trait FromBytes : Sized {
    fn from_bytes(bytes: &[u8]) -> Option<Self>;
}

#[derive(Debug, PartialEq)]
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
                _ => unreachable!(),
            }
        }

        HttpMethod::Unsupported
    }
}

/// A type representing a HTTP header name/value pair. E.g.
///
/// ```no_compile
/// Host: docs.rs:443
/// ```
#[derive(Default, Debug, PartialEq, Clone, Copy)]
pub struct Header<'a>(pub &'a [u8], pub &'a [u8]);

/// A type to represent a HTTP request object
pub struct Request<'header, 'buffer: 'header> {
    /// The object's method - E.g. `GET`, `POST`
    pub method: &'buffer [u8],
    /// The path value
    pub path: &'buffer [u8],
    /// The version string - E.g. `HTTP/1.1`
    pub version: &'buffer [u8],
    /// The headers contained in the object
    pub headers: &'header [Header<'buffer>],
    /// The body of the request
    pub body: &'buffer [u8],
}

impl<'h, 'b: 'h> From<(&'b [u8], &'b [u8], &'b [u8], &'h [Header<'b>], &'b [u8])> for Request<'h, 'b> {
    fn from(parts: (&'b [u8], &'b [u8], &'b [u8], &'h [Header<'b>], &'b [u8])) -> Request<'h, 'b> {
        let (method, path, version, headers, body) = parts;
        Request {
            method: method,
            path: path,
            version: version,
            headers: headers,
            body: body,
        }
    }
}

/// A type respresenting a HTTP response object
pub struct Response<'h, 'b: 'h> {
    /// The version string - E.g. `HTTP/1.1`
    pub version: &'b [u8],
    /// The status code - E.g. `200`, `404`, etc.
    pub status_code: &'b [u8],
    /// The status text - E.g. `OK`, `Not Found`, etc.
    pub status_text: &'b [u8],
    /// The headers contained in the object
    pub headers: &'h [Header<'b>],
    /// The body of the request
    pub body: &'b [u8],
}

impl<'h, 'b: 'h> From<(&'b [u8], &'b [u8], &'b [u8], &'h [Header<'b>], &'b [u8])> for Response<'h, 'b> {
    fn from(parts: (&'b [u8], &'b [u8], &'b [u8], &'h [Header<'b>], &'b [u8])) -> Response<'h, 'b> {
        let (version, status, text, headers, body) = parts;
        Response {
            version: version,
            status_code: status,
            status_text: text,
            headers: headers,
            body: body,
        }
    }
}

