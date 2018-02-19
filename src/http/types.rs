use std::fmt;

use http::parser;

mod v2 {
    use std::fmt;

    use super::HttpMethod;
    use super::to_lower;

    use result::PollResult;
    use pollable::{IntoPollable, Pollable, PollableResult};

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum HttpVersion {
        Http1,
        Http11,
    }

    impl fmt::Display for HttpVersion {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match *self {
                HttpVersion::Http1 => write!(f, "HTTP/1.0"),
                HttpVersion::Http11 => write!(f, "HTTP/1.1"),
            }
        }
    }

    #[derive(Debug)]
    pub struct Header(String, String);

    pub type BodyChunk = Vec<u8>;

    pub struct HeaderIter<'a>(::std::slice::Iter<'a, Header>);

    impl<'a> Iterator for HeaderIter<'a> {
        type Item = (&'a str, &'a str);

        fn next(&mut self) -> Option<Self::Item> {
            self.0.next()
                .map(|h| (&*h.0, &*h.1))
        }
    }

    struct Object<B> {
        version: HttpVersion,
        headers: Vec<Header>,
        body: B,
    }

    impl<B> Object<B> where
        B: Pollable<Item=BodyChunk>
    {
        fn version(&self) -> HttpVersion {
            self.version
        }

        fn add_header(&mut self, name: &str, value: &str) {
            self.headers.push(Header(name.to_owned(), value.to_owned()));
        }

        fn headers(&self) -> HeaderIter {
            HeaderIter(self.headers.iter())
        }

        fn header_value(&self, name: &str) -> Option<&str> {
            self.headers()
                .position(|(n, _)| {
                    n.as_bytes()
                        .iter()
                        .map(|b| to_lower(*b))
                        .eq(name.as_bytes()
                            .iter()
                            .map(|b| to_lower(*b))
                        )
                })
                .map(|i| &*self.headers[i].1)
        }

        fn poll_body(&mut self) -> Result<PollResult<B::Item>, B::Error> {
            self.body.poll()
        }
    }

    impl<B> IntoPollable for Response<B> where
        B: Pollable<Item=BodyChunk>
    {
        type Item = (Self, BodyChunk);
        type Error = B::Error;
        type Pollable = ResponsePollable<B>;

        fn into_pollable(self) -> Self::Pollable {
            ResponsePollable(Some(self))
        }
    }

    pub struct ResponsePollable<B>(Option<Response<B>>);

    impl<B> Pollable for ResponsePollable<B> where
        B: Pollable<Item=BodyChunk>
    {
        type Item = (Response<B>, B::Item);
        type Error = B::Error;
        
        fn poll(&mut self) -> Result<PollResult<Self::Item>, Self::Error> {
            match self.0.take() {
                Some(mut r) => match r.poll_body()? {
                    PollResult::Ready(body) => return Ok(PollResult::Ready((r, body))),
                    PollResult::NotReady => self.0 = Some(r),
                },
                None => panic!("Poll called on finished result"),
            }

            Ok(PollResult::NotReady)
        }
    }

    pub struct Response<B = PollableResult<BodyChunk, ()>> {
        inner: Object<B>,
        status_code: usize,
        status_text: String,
    }

    impl<B> Response<B> where
        B: Pollable<Item=BodyChunk>
    {
        pub fn version(&self) -> HttpVersion {
            self.inner.version()
        }

        pub fn status_code(&self) -> usize {
            self.status_code
        }

        pub fn status_text(&self) ->  &str {
            &*self.status_text
        }

        pub fn add_header(&mut self, name: &str, value: &str) {
            self.inner.add_header(name, value);
        }

        pub fn headers(&self) -> HeaderIter {
            self.inner.headers()
        }

        pub fn header_value(&self, name: &str) -> Option<&str> {
            self.inner.header_value(name)
        }

        pub fn poll_body(&mut self) -> Result<PollResult<B::Item>, B::Error> {
            self.inner.poll_body()
        }
    }

    pub struct Request<B = PollableResult<BodyChunk, ()>> {
        inner: Object<B>,
        method: HttpMethod,
        path: String,
    }

    impl<B> Request<B> where
        B: Pollable<Item=BodyChunk>
    {
        pub fn version(&self) -> HttpVersion {
            self.inner.version()
        }

        pub fn path(&self) -> &str {
            &*self.path
        }

        pub fn method(&self) ->  HttpMethod {
            self.method
        }

        pub fn add_header(&mut self, name: &str, value: &str) {
            self.inner.add_header(name, value);
        }

        pub fn headers(&self) -> HeaderIter {
            self.inner.headers()
        }

        pub fn header_value(&self, name: &str) -> Option<&str> {
            self.inner.header_value(name)
        }
    }

    pub struct ResponseBuilder<'a> {
        version: HttpVersion,
        status_code: usize,
        status_text: &'a str,
    }
    
    impl<'a> ResponseBuilder<'a> {
        pub fn new(status_code: usize, 
                   status_text: &'a str) -> ResponseBuilder<'a>
        {
            ResponseBuilder {
                version: HttpVersion::Http11,
                status_code: status_code,
                status_text: status_text,
            }
        }

        pub fn build(&self) -> Response {
            self._build(Ok(vec![]))
        }

        pub fn build_with_content<T>(&self, t: T) -> Response where
            T: AsRef<[u8]>
        {
            self._build(Ok(t.as_ref().to_vec()))
        }

        pub fn build_with_stream<I>(&self, body: I) -> Response where
                I: IntoIterator<Item=u8>
        {
            self._build(Ok(body.into_iter().collect::<BodyChunk>()))
        }

        fn _build<B>(&self, body: B)
            -> Response<B::Pollable> where
                B: IntoPollable<Item=BodyChunk>
        {
            Response {
                inner: Object {
                    version: self.version,
                    headers: vec![],
                    body: body.into_pollable(),
                },
                status_code: self.status_code,
                status_text: String::from(self.status_text),
            }
        }

        pub fn build_with_pollable<B>(&self, body: B) 
            -> Response<B::Pollable> where
                B: IntoPollable<Item=BodyChunk>
        {
            self._build(body)
        }
    }

    pub struct RequestBuilder<'a> {
        method: HttpMethod,
        path: &'a str,
        version: HttpVersion,
    }
    
    impl<'a> RequestBuilder<'a> {
        pub fn new<M>(method: M, 
                      path: &'a str) -> RequestBuilder<'a> where
            M: Into<HttpMethod>
        {
            RequestBuilder {
                method: method.into(),
                path: path,
                version: HttpVersion::Http11,
            }
        }

        pub fn build(&self) -> Request {
            self.build_with_pollable(Ok(vec![]))
        }

        pub fn build_with_buffer<I>(&self, body: I) -> Request where
                I: IntoIterator<Item=u8>
        {
            self.build_with_pollable(Ok(body.into_iter().collect::<BodyChunk>()))
        }

        pub fn build_with_pollable<B>(&self, body: B) 
            -> Request<B::Pollable> where
                B: IntoPollable<Item=BodyChunk>
        {
            Request {
                inner: Object {
                    version: self.version,
                    headers: vec![],
                    body: body.into_pollable(),
                },
                method: self.method,
                path: String::from(self.path),
            }
        }
    }
}

trait FromBytes : Sized {
    fn from_bytes(bytes: &[u8]) -> Option<Self>;
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum HttpMethod {
    Connect,
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
    Unsupported,
}

fn to_lower(v: u8) -> u8 {
    match v {
        b'A'...b'Z' => v + (b'a' - b'A'),
        o => o
    }
}

fn which_of(to_find: &[u8], in_set: &[&[u8]]) -> Option<usize> {
    for (i, el) in in_set.iter().enumerate() {
        let eq = el.iter().map(|byte| to_lower(*byte))
            .eq(to_find.iter().map(|byte| to_lower(*byte)));

        if eq {
            return Some(i);
        }
    }

    None
}

impl<'a> From<&'a [u8]> for HttpMethod {
    fn from(bytes: &'a [u8]) -> HttpMethod {
        let valid: &[&[u8]] = &[
            b"connect",
            b"Get",
            b"Post",
            b"Put",
            b"Delete",
            b"Patch",
            b"Head",
            b"options",
        ];

        if let Some(n) = which_of(bytes, valid) {
            return match n {
                0 => HttpMethod::Connect,
                1 => HttpMethod::Get,
                2 => HttpMethod::Post,
                3 => HttpMethod::Put,
                4 => HttpMethod::Delete,
                5 => HttpMethod::Patch,
                6 => HttpMethod::Head,
                7 => HttpMethod::Options,
                _ => panic!("Unsupported HTTP method '{}'", 
                            ::std::str::from_utf8(bytes).unwrap()),
            }
        }

        panic!("Unsupported HTTP method '{}'", 
               ::std::str::from_utf8(bytes).unwrap());
//        HttpMethod::Unsupported
    }
}

impl<'a> Into<&'static str> for &'a HttpMethod {
    fn into(self) -> &'static str {
        match *self {
            HttpMethod::Connect => "CONNECT", 
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Head => "HEAD",
            HttpMethod::Options => "OPTIONS",
            o => panic!("Unsupported HTTP method {:?}", o),
        }
    }
}

impl fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", Into::<&'static str>::into(self))
    }
}

fn convert_slice_to_indices<T>(s: &[T], source: &[T]) -> Slice {
    let (sub, source) = {
        ((s.as_ptr() as usize, s.as_ptr() as usize + s.len()),
        (source.as_ptr() as usize, source.as_ptr() as usize + source.len()))
    };

    if (sub.0 < source.0) || (sub.1 > source.1) {
        panic!("Sub-slice is outside the bounds of its source ({}, {}: {}) - ({}, {}: {})",
               sub.0, sub.1, sub.1 - sub.0, source.0, source.1, source.1 - source.0);
    }

    Slice(sub.0 - source.0, sub.1 - source.0) 
}

trait FromParsed<Source> {
    fn from_parsed(source: Source, header: &[u8], body: &[u8]) -> Self;
}

struct Slice(usize, usize);

struct Header {
    name: Slice,
    value: Slice,
}

pub struct HeaderIter<'a>(&'a [u8], ::std::slice::Iter<'a, Header>);

impl<'a> Iterator for HeaderIter<'a> {
    type Item = (&'a [u8], &'a [u8]);

    fn next(&mut self) -> Option<Self::Item> {
        self.1.next()
            .map(|header| (
                &self.0[header.name.0..header.name.1],
                &self.0[header.value.0..header.value.1]
            ))
    }
}

struct DetachedHeaderIter<'a>(&'a [u8], ::std::slice::Iter<'a, Header>);

impl<'a> Iterator for DetachedHeaderIter<'a> {
    type Item = (&'a str, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        use std::str::from_utf8;

        self.1.next()
            .map(|h| (
                from_utf8(&self.0[h.name.0..h.name.1]).unwrap(),
                from_utf8(&self.0[h.value.0..h.value.1]).unwrap()
            ))
    }
}

struct DetachedRequest {
    method: HttpMethod,
    path: Slice,
    version: Slice,
    headers: Vec<Header>,
    body: Slice,
}

impl DetachedRequest {
    fn method(&self) -> HttpMethod {
        self.method
    }

    fn path<'a>(&'a self, buffer: &'a [u8]) -> &'a str {
        ::std::str::from_utf8(
            &buffer[self.path.0..self.path.1]).unwrap()
    }

    fn version<'a>(&'a self, buffer: &'a [u8]) -> &'a str {
        ::std::str::from_utf8(
            &buffer[self.version.0..self.version.1]).unwrap()
    }

    fn headers<'a>(&'a self, buffer: &'a [u8]) -> DetachedHeaderIter<'a> {
        DetachedHeaderIter(buffer, self.headers.iter())
    }
}

struct DetachedResponse {
    version: Slice,
    status_code: Slice,
    status_text: Slice,
    headers: Vec<Header>,
    body: Slice,
}

impl DetachedResponse {
    fn status_code<'a>(&'a self, buffer: &'a [u8]) -> &'a str {
        ::std::str::from_utf8(
            &buffer[self.status_code.0..self.status_code.1]).unwrap()
    }

    fn status_text<'a>(&'a self, buffer: &'a [u8]) -> &'a str {
        ::std::str::from_utf8(
            &buffer[self.status_text.0..self.status_text.1]).unwrap()
    }

    fn version<'a>(&'a self, buffer: &'a [u8]) -> &'a str {
        ::std::str::from_utf8(
            &buffer[self.version.0..self.version.1]).unwrap()
    }

    fn headers<'a>(&'a self, buffer: &'a [u8]) -> DetachedHeaderIter<'a> {
        DetachedHeaderIter(buffer, self.headers.iter())
    }
}

pub use self::v2::{
    BodyChunk, 
    Request, 
    RequestBuilder, 
    Response, 
    ResponseBuilder
};

impl<'h, 'b: 'h> FromParsed<parser::Request<'h, 'b>> for DetachedRequest {
    fn from_parsed(source: parser::Request<'h, 'b>, 
                   header: &[u8],
                   body: &[u8]) -> DetachedRequest
    {
        let method = source.method().into();
        let path = convert_slice_to_indices(source.path(), header);
        let version = convert_slice_to_indices(source.version(), header);
        let headers = source.headers().iter()
            .map(|h| Header {
                name: convert_slice_to_indices(h.0, header),
                value: convert_slice_to_indices(h.1, header),
            })
            .collect::<Vec<_>>();
        let body = convert_slice_to_indices(body, header);

        DetachedRequest {
            method: method,
            path: path,
            version: version,
            headers: headers,
            body: body,
        }
    }
}

impl<'h, 'b: 'h> FromParsed<parser::Response<'h, 'b>> for DetachedResponse {

    fn from_parsed(source: parser::Response<'h, 'b>,
                   header: &[u8],
                   body: &[u8]) -> DetachedResponse
    {
        let version = convert_slice_to_indices(source.version(), header);
        let status_code = convert_slice_to_indices(source.status_code(), header);
        let status_text = convert_slice_to_indices(source.status_text(), header);
        let headers = source.headers().iter()
            .map(|h| Header {
                name: convert_slice_to_indices(h.0, header),
                value: convert_slice_to_indices(h.1, header),
            })
            .collect::<Vec<_>>();
        let body = convert_slice_to_indices(body, header);

        DetachedResponse {
            version: version,
            status_code: status_code,
            status_text: status_text,
            headers: headers,
            body: body,
        }
    }
}

pub fn parse_request(buffer: &mut Vec<u8>) -> Option<Request> {
    use std::str::from_utf8;

    let (r, consumed) = {
        let mut headers = [parser::Header::default(); 32];
        let mut request = parser::Request::new(&mut headers);
        //  TODO:
        //      Properly parse the body...
        if let Some(n) = request.parse(buffer) {
            (DetachedRequest::from_parsed(request, buffer, &buffer[n..n]), n)
        }
        else {
            return None;
        }
    };

    let mut request = 
        RequestBuilder::new(r.method(), r.path(buffer))
            .build();

    for (name, value) in r.headers(buffer) {
        request.add_header(name, value);
    }
    
    buffer.drain(..consumed);
    Some(request)
}

pub fn parse_response(buffer: &mut Vec<u8>) -> Option<Response> {
    use std::str::from_utf8;

    let (r, consumed) = {
        let mut headers = [parser::Header::default(); 32];
        let mut response = parser::Response::new(&mut headers);
        //  TODO:
        //      Properly parse the body...
        if let Some(n) = response.parse(buffer) {
            (DetachedResponse::from_parsed(response, buffer, &buffer[n..n]), n)
        }
        else {
            return None;
        }
    };

    let mut response = 
        ResponseBuilder::new(r.status_code(buffer).parse().unwrap(), 
                             r.status_text(buffer))
            .build();

    for (name, value) in r.headers(buffer) {
        response.add_header(name, value);
    }
    
    buffer.drain(..consumed);
    Some(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_header() {
        let mut buffer = b"HTTP/1.1 200 Ok\r\n\
            Host: www.someserver.com\r\n\
            \r\n\
            Hello, World!".to_vec();

        let mut r = parse_response(&mut buffer).unwrap();
        r.add_header("Accept", "text/json");
        r.add_header("X-Some-Header", "1234567890");

        assert_eq!(3, r.headers().count());
        assert_eq!(
            ("Accept".as_ref(), "text/json".as_ref()), 
            r.headers().nth(1).unwrap()
        );

        assert_eq!(
            ("X-Some-Header".as_ref(), "1234567890".as_ref()), 
            r.headers().nth(2).unwrap()
        );
    }

    #[test]
    fn convert_a_parsed_request() {
        let mut buffer = b"GET /a HTTP/1.1\r\n\
Accept: text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8\r\n\
Accept-Encoding: gzip, deflate\r\n\
Accept-Language: en-US,en;q=0.5\r\n\r\n".to_vec();

        let r = parse_request(&mut buffer).unwrap();

        assert_eq!(HttpMethod::Get, r.method());
        assert_eq!("/a", r.path());
        assert_eq!(v2::HttpVersion::Http11, r.version());
        assert_eq!(
            ("Accept-Encoding".as_ref(), "gzip, deflate".as_ref()), 
            r.headers().nth(1).unwrap()
        );
        println!("{}", ::std::str::from_utf8(&*buffer).unwrap());
        assert_eq!(b"", &*buffer);
    }

    #[test]
    fn convert_a_parsed_response() {
        let mut buffer = b"HTTP/1.1 404 Not found\r\n\
            Host: www.someserver.com\r\n\
            \r\n\
            Hello, World!".to_vec();

        let r = parse_response(&mut buffer).unwrap();

        assert_eq!(v2::HttpVersion::Http11, r.version());
        assert_eq!(404, r.status_code());
        assert_eq!("Not found", r.status_text());
        assert_eq!(
            ("Host".as_ref(), "www.someserver.com".as_ref()), 
            r.headers().next().unwrap()
        );
        assert_eq!(b"Hello, World!", &*buffer);
    }
}
