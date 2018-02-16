use std::collections::HashSet;

#[derive(Debug, PartialEq)]
pub enum Part {
    Exact(String),
    Param(String),
    Missing,
}

type Parameters<'a> = Vec<(&'a str, &'a str)>;

pub struct Pattern(Vec<Part>);

#[derive(Debug, PartialEq)]
pub struct NoMatchError;

impl Pattern {
    pub fn new(pattern: &str) -> Pattern {
        let parts = pattern.split('/')
            .filter(|p| p.len() != 0 && *p != ":")
            .map(|p| match p.starts_with(":") {
                true => Part::Param(String::from(&p[1..])),
                false => Part::Exact(String::from(p)),
            })
            .collect::<Vec<_>>();

        Pattern(parts)
    }

    pub fn parts(&self) -> ::std::slice::Iter<Part> {
        self.0.iter()
    }

    pub fn match_uri<'a>(&'a self, uri: &'a str) 
        -> Result<Parameters<'a>, NoMatchError> 
    {
        use std::iter;

        let uri_end_pos = uri.chars()
            .position(|c| c == '?' || c == '#')
            .unwrap_or_else(|| uri.len());

        (&uri[..uri_end_pos]).split("/")
            .filter(|p| p.len() != 0)
            .zip(self.parts().chain(iter::repeat(&Part::Missing)))
            .filter_map(|(uri, part)| {
                if let Part::Missing = *part {
                    return Some(Err(NoMatchError));
                }

                match *part {
                    Part::Exact(ref u) if uri == u => None,
                    Part::Param(ref p) => Some(Ok((p.as_ref(), uri))),
                    _ => Some(Err(NoMatchError)),
                }
            })
            .collect::<_>()
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
    fn match_uri() {
        let p = Pattern::new("/api/:item");
        let params = p.match_uri("/api/resource?_filter=hello+world");
        assert!(params.is_ok());
        assert_eq!(("item", "resource"), params.unwrap()[0]);
    }
}
