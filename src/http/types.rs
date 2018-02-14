use http::parser;

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
                _ => panic!("Unsupported HTTP method"),
            }
        }

        HttpMethod::Unsupported
    }
}

fn convert_slice<T>(s: &[T], source: &[T]) -> Slice {
    Slice( 
        (s.as_ptr() as usize) - (source.as_ptr() as usize ),
        (s.as_ptr() as usize + s.len()) - (source.as_ptr() as usize)
    )
}

trait FromParsed<Source> {
    fn from_parsed(source: Source, buffer: &[u8]) -> Self;
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

struct DetachedRequest {
    method: HttpMethod,
    path: Slice,
    version: Slice,
    headers: Vec<Header>,
}

impl DetachedRequest {
    fn bind_buffer(self, buffer: Vec<u8>) -> Request {
        let byte_length = self.headers.last()
            .map(|h| h.value.1)
            .unwrap_or_else(|| self.version.1);

        Request {
            inner: self,
            buffer: buffer,
        }
    }
}

struct DetachedResponse {
    version: Slice,
    status_code: Slice,
    status_text: Slice,
    headers: Vec<Header>,
}

impl DetachedResponse {
    fn bind_buffer(self, buffer: Vec<u8>) -> Response {
        let byte_length = self.headers.last()
            .map(|h| h.value.1)
            .unwrap_or_else(|| self.status_text.1);

        Response {
            inner: self,
            buffer: buffer,
        }
    }
}

pub struct Response {
    inner: DetachedResponse,
    buffer: Vec<u8>,
}

impl Response {
    pub fn version(&self) -> &[u8] {
        &self.buffer[self.inner.version.0..self.inner.version.1]
    }

    pub fn status_code(&self) -> &[u8] {
        &self.buffer[self.inner.status_code.0..self.inner.status_code.1]
    }

    pub fn status_text(&self) -> &[u8] {
        &self.buffer[self.inner.status_text.0..self.inner.status_text.1]
    }

    pub fn headers(&self) -> HeaderIter {
        HeaderIter(&self.buffer, self.inner.headers.iter())
    }
}

pub struct Request {
    inner: DetachedRequest,
    buffer: Vec<u8>,
}

impl Request {
    pub fn method(&self) -> HttpMethod {
        self.inner.method
    }

    pub fn path(&self) -> &[u8] {
        &self.buffer[self.inner.path.0..self.inner.path.1]
    }

    pub fn version(&self) -> &[u8] {
        &self.buffer[self.inner.version.0..self.inner.version.1]
    }

    pub fn headers(&self) -> HeaderIter {
        HeaderIter(&self.buffer, self.inner.headers.iter())
    }
}

impl<'h, 'b: 'h> FromParsed<parser::Request<'h, 'b>> for DetachedRequest {
    fn from_parsed(source: parser::Request<'h, 'b>, 
                   buffer: &[u8]) -> DetachedRequest
    {
        let method = source.method().into();
        let path = convert_slice(source.path(), buffer);
        let version = convert_slice(source.version(), buffer);
        let headers = source.headers().iter()
            .map(|h| Header {
                name: convert_slice(h.0, buffer),
                value: convert_slice(h.1, buffer),
            })
            .collect::<Vec<_>>();

        DetachedRequest {
            method: method,
            path: path,
            version: version,
            headers: headers,
        }
    }
}

impl<'h, 'b: 'h> FromParsed<parser::Response<'h, 'b>> for DetachedResponse {

    fn from_parsed(source: parser::Response<'h, 'b>,
                   buffer: &[u8]) -> DetachedResponse
    {
        let version = convert_slice(source.version(), buffer);
        let status_code = convert_slice(source.status_code(), buffer);
        let status_text = convert_slice(source.status_text(), buffer);
        let headers = source.headers().iter()
            .map(|h| Header {
                name: convert_slice(h.0, buffer),
                value: convert_slice(h.1, buffer),
            })
            .collect::<Vec<_>>();

        DetachedResponse {
            version: version,
            status_code: status_code,
            status_text: status_text,
            headers: headers,
        }
    }
}

pub fn parse_request(buffer: &mut Vec<u8>) -> Option<Request> {
    let (request, consumed) = {
        let mut headers = [parser::Header::default(); 32];
        let mut request = parser::Request::new(&mut headers);
        if let Some(n) = request.parse(buffer) {
            (DetachedRequest::from_parsed(request, buffer), n)
        }
        else {
            return None;
        }
    };

    Some(request.bind_buffer(buffer.drain(..consumed).collect()))
}

pub fn parse_response(buffer: &mut Vec<u8>) -> Option<Response> {
    let (response, consumed) = {
        let mut headers = [parser::Header::default(); 32];
        let mut response = parser::Response::new(&mut headers);
        if let Some(n) = response.parse(buffer) {
            (DetachedResponse::from_parsed(response, buffer), n)
        }
        else {
            return None;
        }
    };

    Some(response.bind_buffer(buffer.drain(..consumed).collect()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_a_parsed_request() {
        let mut buffer = b"GET /index.html HTTP/1.1\r\n\
            Host: www.someserver.com\r\n\
            \r\n\
            Hello, World!".to_vec();

        let r = parse_request(&mut buffer).unwrap();

        assert_eq!(HttpMethod::Get, r.method());
        assert_eq!(b"/index.html", r.path());
        assert_eq!(b"HTTP/1.1", r.version());
        assert_eq!(
            (b"Host".as_ref(), b"www.someserver.com".as_ref()), 
            r.headers().next().unwrap()
        );
        assert_eq!(b"Hello, World!", &*buffer);
    }

    #[test]
    fn convert_a_parsed_response() {
        let mut buffer = b"HTTP/1.1 404 Not found\r\n\
            Host: www.someserver.com\r\n\
            \r\n\
            Hello, World!".to_vec();

        let r = parse_response(&mut buffer).unwrap();

        assert_eq!(b"HTTP/1.1", r.version());
        assert_eq!(b"404", r.status_code());
        assert_eq!(b"Not found", r.status_text());
        assert_eq!(
            (b"Host".as_ref(), b"www.someserver.com".as_ref()), 
            r.headers().next().unwrap()
        );
        assert_eq!(b"Hello, World!", &*buffer);
    }
}
