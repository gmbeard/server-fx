use std::collections::HashSet;

use http::types;

#[derive(Debug, PartialEq)]
pub enum Part {
    Exact(String),
    Param(String),
    Wildcard,
    Missing,
}

pub type Parameters<'a> = Vec<(&'a str, String)>;

pub struct Pattern(Vec<Part>, bool);

#[derive(Debug, PartialEq)]
pub struct NoMatchError;

impl Pattern {
    pub fn new(pattern: &str) -> Pattern {
        let mut has_wildcard = false;
        let parts = pattern.split('/')
            .filter(|p| p.len() != 0 && *p != ":")
            .map(|p| {
                has_wildcard = p == "*";
                if has_wildcard {
                    return Part::Wildcard;
                }

                match p.starts_with(":") {
                    true => Part::Param(String::from(&p[1..])),
                    false => Part::Exact(String::from(p)),
                }
            })
            .collect::<Vec<_>>();

        Pattern(parts, has_wildcard)
    }

    fn parts(&self) -> ::std::slice::Iter<Part> {
        self.0.iter()
    }

    pub fn match_uri<'a, 'b>(&'a self, uri: &'b str) 
        -> Result<Parameters<'a>, NoMatchError> 
    {
        use std::iter;

        let uri_end_pos = uri.chars()
            .position(|c| c == '?' || c == '#')
            .unwrap_or_else(|| uri.len());

        let chain = if self.1 {
            iter::repeat(&Part::Wildcard)
        }
        else {
            iter::repeat(&Part::Missing)
        };

        (&uri[..uri_end_pos]).split("/")
            .filter(|p| p.len() != 0)
            .zip(self.parts().chain(chain))
            .filter_map(|(uri, part)| {
                if let Part::Missing = *part {
                    return Some(Err(NoMatchError));
                }

                match *part {
                    Part::Exact(ref u) if uri == u => None,
                    Part::Wildcard => None,
                    Part::Param(ref p) => Some(Ok((p.as_ref(), String::from(uri)))),
                    _ => Some(Err(NoMatchError)),
                }
            })
            .collect::<_>()
    }
}

pub trait RouteHandler {
    fn handle<'a>(&'a self, 
                  request: types::Request, 
                  params: &Parameters<'a>) 
        -> types::Response;
}

pub enum HandleRouteResult<T, U> {
    Handled(T),
    NotHandled(U),
}

pub struct Route {
    method: types::HttpMethod,
    pattern: Pattern,
    handler: Box<RouteHandler>,
}

impl Route {
    pub fn new<H>(method: types::HttpMethod, 
                  uri_pat: &str, 
                  handler: H) -> Route where
        H: RouteHandler + 'static
    {
        Route {
            method: method,
            pattern: Pattern::new(uri_pat),
            handler: Box::new(handler)
        }
    }

    pub fn handle(&self, 
                  request: types::Request) 
        -> HandleRouteResult<types::Response, types::Request>
    {
        use self::HandleRouteResult::*;

        if request.method() != self.method {
            return NotHandled(request);
        }

        match self.pattern.match_uri(request.path()) {
            Ok(params) => Handled(self.handler.handle(request, &params)),
            Err(_) => NotHandled(request),
        }
    }
}

#[cfg(test)]
mod route_should {
    use super::*;

    #[test]
    fn compile_pattern() {
        let p = Pattern::new("/api/:item");

        let mut pattern_iter = p.parts();

        assert_eq!(Some(&Part::Exact("api".to_owned())), pattern_iter.next());
        assert_eq!(Some(&Part::Param("item".to_owned())), pattern_iter.next());
    }

    #[test]
    fn match_wildcard() {
        let p = Pattern::new("/static/*");
        assert!(p.1);

        assert!(p.match_uri("/static/css/site.css").is_ok());
    }

    #[test]
    fn match_uri() {
        let p = Pattern::new("/api/:item");
        let params = p.match_uri("/api/resource?_filter=hello+world");
        assert!(params.is_ok());
        assert_eq!(("item", "resource".to_string()), params.unwrap()[0]);
    }
}
