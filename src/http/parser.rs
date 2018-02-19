use std::mem;

fn header_line_is_empty(data: &[u8]) -> bool {
    (data.len() > 0 && data[0] == b'\n') ||
        (data.len() > 1 && data[0] == b'\r' && data[1] == b'\n')
}

fn skip_newline(data: &[u8]) -> &[u8] {
    let mut to_skip = 0;
    data.iter()
        .position(|b| *b == b'\r')
        .map(|p| {
            to_skip = p + 1;
        });
    data.iter()
        .position(|b| *b == b'\n')
        .map(|p| {
            to_skip = p + 1;
        });

    &data[to_skip..]
}

fn skip_whitespace(data: &[u8]) -> &[u8] {
    data.iter()
        .position(|byte| *byte != b' ' && *byte != b'\t')
        .map(|p| {
            let (_, tail) = data.split_at(p);
            tail
        })
        .unwrap_or_else(|| {
            let last = data.len();
            &data[0..0]
        })
}

fn skip_header_separator(data: &[u8]) -> &[u8] {
    data.iter()
        .position(|byte| *byte != b'\t' && *byte != b' ' && *byte != b':')
        .map(|p| {
            let (_, tail) = data.split_at(p);
            tail
        })
        .unwrap_or_else(|| {
            let last = data.len();
            &data[0..0]
        })
}

fn split_at_first_newline(data: &[u8]) -> Option<(&[u8], &[u8])> {
    data.iter()
        .position(|byte| *byte == b'\r' || *byte == b'\n')
        .map(|p| data.split_at(p))
}

fn split_at_first_whitespace(data: &[u8]) -> Option<(&[u8], &[u8])> {
    data.iter()
        .position(|byte| *byte == b' ' || *byte == b'\t')
        .map(|p| data.split_at(p))
}

fn split_at_first_header_separator(data: &[u8]) -> Option<(&[u8], &[u8])> {
    data.iter()
        .position(|byte| *byte == b':')
        .map(|p| data.split_at(p))
}

/// A type to parse the *protocol line* of a HTTP request.
/// E.g.
///
/// ```no_compile
/// CONNECT docs.rs:443 HTTP/1.1
/// ```
///
/// `ProtocolParser` is non-allocating and works purely
/// on borrowed data, hence the lifetime parameter.
pub enum ProtocolParser<'a> {
    #[doc(hidden)]
    Method(&'a [u8]),
    #[doc(hidden)]
    Path(&'a [u8], &'a [u8]),
    #[doc(hidden)]
    Version(&'a [u8], &'a [u8], &'a [u8]),
    #[doc(hidden)]
    Done,
}

/// A type to parse a *header* of a HTTP request.
/// E.g.
///
/// ```no_compile
/// Content-Type: text/json; charset=utf-8
/// ```
///
/// `HeaderParser` is non-allocating and works purely
/// on borrowed data, hence the lifetime parameter.
pub enum HeaderParser<'a> {
    #[doc(hidden)]
    Name(&'a [u8]),
    #[doc(hidden)]
    Value(&'a [u8], &'a [u8]),
    #[doc(hidden)]
    Done,
}

impl<'a> ProtocolParser<'a> {
    /// Creates a new instance. `bytes` must be at the start
    /// of the *protocol line* for any parsing to be successful.
    pub fn new(bytes: &'a [u8]) -> ProtocolParser<'a> {
        ProtocolParser::Method(bytes)
    }

    /// Parses the protocol line contained at the start of 
    /// the data provided to [`ProtocolParser::new`]
    ///
    /// Parse requires `&mut self` because it is internally
    /// represented as a state machine and so must modify
    /// itself in the process of parsing.
    ///
    /// # Return Value
    /// If parsing is successful, a tuple is returned consisting
    /// of `(method: HttpMethod, path: &[u8], version: &[u8], 
    /// remaining: &[u8])`. `remaining` is any remaining data found 
    /// after the protocol line. The parser consumes the trailing `\r\n` 
    /// bytes of the protocol line so, assuming a well-formed request, 
    /// `remaining` is at the very start of the first header line.
    ///
    /// If parsing can't be completed because either the data is
    /// incomplete, or it is invalid, then this function returns
    /// `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use server_fx::http::parser::ProtocolParser;
    /// use server_fx::http::types::HttpMethod;
    ///
    /// const HTTP: &'static [u8] = b"GET /index.html HTTP/1.1\r\n";
    ///
    /// let mut parser = ProtocolParser::new(HTTP);
    /// let (method, path, version, tail) = parser.parse().unwrap();
    ///
    /// assert_eq!(HttpMethod::Get, method.into());
    /// assert_eq!(b"/index.html", path);
    /// assert_eq!(b"HTTP/1.1", version);
    /// assert_eq!(0, tail.len());
    /// ```
    ///
    /// [`ProtocolParser::new`]: enum.ProtocolParser.html#method.new
    pub fn parse(&mut self) -> Option<(&'a [u8], &'a [u8], &'a [u8], &'a [u8])> {
        use self::ProtocolParser::*;
        loop {
            let next = match mem::replace(self, Done) {
                Method(data) => {
                    split_at_first_whitespace(data)
                        .map(|(val, tail)| {
                            Path(val, skip_whitespace(tail))
                        })
                },
                Path(method, data) => {
                    split_at_first_whitespace(data)
                        .map(|(val, tail)| {
                            Version(method, val, skip_whitespace(tail))
                        })
                },
                Version(method, url, data) => {
                    return split_at_first_newline(data)
                        .map(|(val, tail)| {
                            (method, url, val, skip_newline(tail))
                        });
                },
                Done => panic!("parse called after done"),
            };

            if let Some(next) = next {
                *self = next;
            }
            else {
                return None
            }
        }
    }
}

impl<'a> HeaderParser<'a> {
    /// Creates a new instance. `bytes` must be at the start
    /// of the *header line* for any parsing to be successful.
    pub fn new(bytes: &'a [u8]) -> HeaderParser<'a> {
        HeaderParser::Name(bytes)
    }

    /// Parses a single HTTP header contained at the start of 
    /// the data provided to [`HeaderParser::new`]
    ///
    /// Parsing requires `&mut self` because it is internally
    /// represented as a state machine and so must modify
    /// itself in the process of parsing.
    ///
    /// # Return Value
    /// If parsing is successful, a tuple is returned consisting
    /// of `(header: Header, remaining: &[u8])`. `remaining` is 
    /// any remaining data found after the protocol line. The parser 
    /// consumes the trailing `\r\n` bytes of the protocol line so, 
    /// assuming a well-formed request, `remaining` is at the very start 
    /// of the next header line.
    ///
    /// If parsing can't be completed because either the data is
    /// incomplete, or it is invalid, then this function returns
    /// `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use server_fx::http::parser::Header;
    /// use server_fx::http::parser::HeaderParser;
    ///
    /// const HTTP: &'static [u8] = b"Content-Type: text/xml; charset=utf8\r\n";
    ///
    /// let mut parser = HeaderParser::new(HTTP);
    /// let (Header (name, value), remaining) = parser.parse().unwrap();
    ///
    /// assert_eq!(b"Content-Type", name);
    /// assert_eq!(b"text/xml; charset=utf8", value);
    /// assert_eq!(0, remaining.len());
    /// ```
    ///
    /// [`HeaderParser::new`]: enum.HeaderParser.html#method.new
    pub fn parse(&mut self) -> Option<(Header<'a>, &'a [u8])> {
        use self::HeaderParser::*;

        loop {
            let next = match mem::replace(self, Done) {
                Name(data) => {
                    if header_line_is_empty(data) {
                        return Some((Header(&data[0..0], &data[0..0]), skip_newline(data)));
                    }

                    split_at_first_header_separator(data)
                        .map(|(val, tail)| {
                            Value(val, skip_header_separator(tail))
                        })
                },
                Value(name, data) => {
                    return split_at_first_newline(data)
                        .map(|(val, tail)| {
                            (Header(name, val), skip_newline(tail))
                        });
                },
                Done => panic!("parse called on finished result"),
            };

            if let Some(next) = next {
                *self = next;
            }
            else {
                return None;
            }
        }
    }
}

struct Object<'headers, 'buffer: 'headers> {
    version: Option<&'buffer [u8]>,
    headers: &'headers mut [Header<'buffer>],
}

impl<'h, 'b: 'h> Object<'h, 'b> {
    fn version(&self) -> &[u8] {
        self.version.as_ref()
            .map(|v| &**v)
            .expect("'version' is empty")
    }

    fn headers(&self) -> &[Header<'b>] {
        self.headers
    }
}

impl<'h, 'b: 'h> Object<'h, 'b> {
    fn new(headers: &'h mut [Header<'b>]) -> Object<'h, 'b> {
        Object {
            version: None,
            headers: headers,
        }
    }

    fn read_headers(&mut self, 
                    data: &'b [u8], 
                    header_data: &'b [u8]) -> Option<usize>
    {
        use std::mem::transmute;

        let mut parser = HeaderParser::new(header_data);
        let mut header_idx = 0;
        let mut bytes_parsed = 
            (header_data.as_ptr() as usize) - 
            (data.as_ptr() as usize);

        while let Some((Header(name, val), tail)) = parser.parse() {
            bytes_parsed = (tail.as_ptr() as usize) - 
                           (data.as_ptr() as usize);

            if name.len() == 0 {
                self.headers =  unsafe { 
                    transmute(&mut self.headers[..header_idx])
                };

                return Some(bytes_parsed)
            }

            if header_idx >= self.headers.len() {
                panic!("Not enough space for headers!");
            }

            self.headers[header_idx] = Header(name, val);
            header_idx += 1;

            parser = HeaderParser::new(tail);
        }

        None
    }

    fn parse<F>(&mut self, data: &'b [u8], mut f: F) -> Option<usize>
        where F: FnMut(&'b [u8], &'b [u8], &'b [u8]) -> Option<&'b [u8]>
    {
        ProtocolParser::new(data).parse()
            .map(|(part1, part2, part3, tail)| {
                self.version = f(part1, part2, part3);
                tail
            })
            .and_then(|header_data| 
                 self.read_headers(data, header_data)
            )
    }
}

/// A type representing a HTTP header name/value pair. E.g.
///
/// ```no_compile
/// Host: docs.rs:443
/// ```
#[derive(Default, PartialEq, Clone, Copy)]
pub struct Header<'a>(pub &'a [u8], pub &'a [u8]);

impl<'a> ::std::fmt::Debug for Header<'a> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        use std::str::from_utf8;

        write!(f, "{}: {}\r\n",
              from_utf8(self.0).unwrap(),
              from_utf8(self.1).unwrap())
    }
}

pub struct Request<'headers, 'buffer: 'headers> {
    #[doc(hidden)]
    method: Option<&'buffer [u8]>,
    #[doc(hidden)]
    path: Option<&'buffer [u8]>,
    #[doc(hidden)]
    object: Object<'headers, 'buffer>,
}

impl<'h, 'b: 'h> ::std::fmt::Debug for Request<'h, 'b> {

    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        use std::str::from_utf8;
        write!(f, "{} {} {}\r\n", 
               from_utf8(self.method()).unwrap(),
               from_utf8(self.path()).unwrap(),
               from_utf8(self.version()).unwrap());
        for h in self.headers() {
            write!(f, "{:?}", h);
        }
        Ok(())
    }
}

impl<'h, 'b: 'h> Request<'h, 'b> {
    pub fn method(&self) -> &[u8] {
        self.method.as_ref()
            .map(|v| &**v)
            .expect("'method' is empty")
    }

    pub fn path(&self) -> &[u8] {
        self.path.as_ref()
            .map(|v| &**v)
            .expect("'path' is empty")
    }

    pub fn version(&self) -> &[u8] {
        self.object.version()
    }

    pub fn headers(&self) -> &[Header<'b>] {
        self.object.headers()
    }
}

impl<'h, 'b: 'h> Request<'h, 'b> {
    pub fn new(headers: &'h mut [Header<'b>]) -> Request<'h, 'b> {
        Request {
            method: None,
            path: None,
            object: Object::new(headers),
        }
    }

    pub fn parse(&mut self, data: &'b [u8]) -> Option<usize> {
        let mut method = None;
        let mut path = None;

        self.object.parse(data, |part1, part2, part3| {
            method = Some(part1);
            path = Some(part2);
            Some(part3)
        })
        .map(|n| {
            self.method = method;
            self.path = path;
            n
        })
    }
}

pub struct Response<'headers, 'buffer: 'headers> {
    status_code: Option<&'buffer [u8]>,
    status_text: Option<&'buffer [u8]>,
    object: Object<'headers, 'buffer>,
}

impl<'h, 'b: 'h> Response<'h, 'b> {
    pub fn status_code(&self) -> &[u8] {
        self.status_code
            .as_ref()
            .map(|v| &**v)
            .expect("'status_code' is empty")
    }

    pub fn status_text(&self) -> &[u8] {
        self.status_text
            .as_ref()
            .map(|v| &**v)
            .expect("'status_text' is empty")
    }

    pub fn version(&self) -> &[u8] {
        self.object.version()
    }

    pub fn headers(&self) -> &[Header<'b>] {
        self.object.headers()
    }
}

impl<'h, 'b: 'h> Response<'h, 'b> {
    pub fn new(headers: &'h mut [Header<'b>]) -> Response<'h, 'b> {
        Response {
            status_code: None,
            status_text: None,
            object: Object::new(headers),
        }
    }

    pub fn parse(&mut self, data: &'b [u8]) -> Option<usize> {
        let mut status_code = None;
        let mut status_text = None;

        self.object.parse(data, |part1, part2, part3| {
            status_code = Some(part2);
            status_text = Some(part3);
            Some(part1)
        })
        .map(|n| {
            self.status_code = status_code;
            self.status_text = status_text;
            n
        })
    }
}

#[cfg(test)]
mod protocol_parser_should {
    use super::*;
    use std::str;
    use http::types::HttpMethod;

    #[test]
    fn parse_protocol_header() {
        let proxy_connect = include_bytes!("../../tests/proxy_connect.txt");
        let mut p = ProtocolParser::new(proxy_connect);
        let (method, url, version, _) = p.parse().unwrap();

        assert_eq!(HttpMethod::Connect, method.into());
        assert_eq!("docs.rs:443", str::from_utf8(url).unwrap());
        assert_eq!("HTTP/1.1", str::from_utf8(version).unwrap());
    }
}

#[cfg(test)]
mod header_parser_should {
    use super::*;
    use std::str;
   
    #[test]
    fn parse_multiple_headers() {
        let proxy_connect = include_bytes!("../../tests/proxy_connect.txt");
        let (_, headers) = split_at_first_newline(proxy_connect).unwrap();
        let headers = skip_newline(headers);

        let mut p = HeaderParser::new(headers);
        let (Header(name, val), tail) = p.parse().unwrap();

        assert_eq!("User-Agent", str::from_utf8(name).unwrap());
        assert_eq!(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:59.0) \
            Gecko/20100101 Firefox/59.0", str::from_utf8(val).unwrap());

        let mut p = HeaderParser::new(tail);
        let (Header(name, val), tail) = p.parse().unwrap();

        assert_eq!("Proxy-Connection", str::from_utf8(name).unwrap());
        assert_eq!(
            "keep-alive", str::from_utf8(val).unwrap());

        let mut p = HeaderParser::new(tail);
        let (Header(name, val), tail) = p.parse().unwrap();

        assert_eq!("Connection", str::from_utf8(name).unwrap());
        assert_eq!(
            "keep-alive", str::from_utf8(val).unwrap());

        let mut p = HeaderParser::new(tail);
        let (Header(name, val), tail) = p.parse().unwrap();

        assert_eq!("Host", str::from_utf8(name).unwrap());
        assert_eq!(
            "docs.rs:443", str::from_utf8(val).unwrap());

        let (Header(_, _), tail) = HeaderParser::new(tail).parse().unwrap();
        assert_eq!("Hello, World!\r\n", str::from_utf8(tail).unwrap());

    }

    #[test]
    fn parse_a_header() {
        let proxy_connect = include_bytes!("../../tests/proxy_connect.txt");
        let (_, headers) = split_at_first_newline(proxy_connect).unwrap();
        let headers = skip_newline(headers);

        let mut p = HeaderParser::new(headers);
        let (Header(name, val), _) = p.parse().unwrap();

        assert_eq!("User-Agent", str::from_utf8(name).unwrap());
        assert_eq!(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:59.0) \
            Gecko/20100101 Firefox/59.0", str::from_utf8(val).unwrap());
    }
}

#[cfg(test)]
mod request_parser_should {
    use super::*;
    use std::str;
    use http::types;

    #[test]
    fn parse_a_request() {
        let proxy_connect = include_bytes!("../../tests/proxy_connect.txt");
        let mut header_size = 16;
        loop {
            let mut headers = vec![Header::default(); header_size];
            let mut parser = Request::new(&mut headers);
            if let Some(_) = parser.parse(proxy_connect) {

                assert_eq!(types::HttpMethod::Connect, parser.method().into());
                assert_eq!("docs.rs:443", str::from_utf8(parser.path()).unwrap());
                assert_eq!(4, parser.headers().len());
//                assert_eq!("Hello, World!\r\n", str::from_utf8(r.body).unwrap());
                break;
            }

            header_size *= 2;
        }

    }
}

#[cfg(test)]
mod request_should {
    use super::*;
    use http::types::HttpMethod;

    #[test]
    fn parse_successfully() {
        let proxy_connect = include_bytes!("../../tests/proxy_connect.txt");
        const HEADER_SIZE: usize = 16;
        let mut headers = [Header::default(); HEADER_SIZE];
        let mut parser = Request::new(&mut headers);

        assert!(parser.parse(proxy_connect).is_some());
        assert_eq!(HttpMethod::Connect, parser.method().into());
    }

    #[test]
    fn parse_with_zero_headers() {
        let request = b"POST / HTTP/1.1\r\n\r\nHello, World!";
        const HEADER_SIZE: usize = 16;
        let mut headers = [Header::default(); HEADER_SIZE];
        let mut parser = Request::new(&mut headers);
        let result = parser.parse(request);
        assert!(result.is_some());
        assert_eq!(0, parser.headers().len());

        assert_eq!(b"Hello, World!", &request[result.unwrap()..]);
    }
}
