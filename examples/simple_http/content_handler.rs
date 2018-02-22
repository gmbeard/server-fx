extern crate pulldown_cmark;

use std::path::PathBuf;
use std::io::Read;

use server_fx::http::router::{Parameters, RouteHandler};
use server_fx::http::types::{Request, Response, ResponseBuilder};

use self::pulldown_cmark::{html, Parser};

pub struct ContentRouteHandler {
    base_path: PathBuf,
}

impl ContentRouteHandler {
    pub fn new<P: Into<PathBuf>>(base_path: P) -> ContentRouteHandler {
        ContentRouteHandler {
            base_path: base_path.into(),
        }
    }
}

fn get_param_value<'a>(name: &'a str, params: &'a Parameters) -> Option<&'a str> {
    params.iter().position(|n| n.0 == name)
        .map(|n| &*params[n].1)
}

impl RouteHandler for ContentRouteHandler {

    fn handle(&self, _: Request, params: &Parameters) -> Response {
        let path = match get_param_value("page", params) {
            Some(v) => self.base_path.join(format!("{}.md", v)),
            None => {
                return ResponseBuilder::new(404, "Not found")
                    .build();
            }
        };

        if !path.exists() {
                return ResponseBuilder::new(404, "Not found")
                    .build();
        }

        let mut html_buf = String::new();
        let mut data_buf = vec![];
        ::std::fs::File::open(path)
            .unwrap()
            .read_to_end(&mut data_buf)
            .unwrap();

        let parser = Parser::new(::std::str::from_utf8(&data_buf).unwrap());
        html::push_html(&mut html_buf, parser);

        let mut resp = ResponseBuilder::new(200, "OK")
            .build_with_stream(html_buf.into_bytes());

        resp.add_header("Content-Type", "text/html");

        resp
    }
}
