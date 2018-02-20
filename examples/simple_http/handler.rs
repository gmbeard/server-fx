use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::ffi::OsStr;

use server_fx::handler::Handler;
use server_fx::http::types;
use server_fx::pollable::{IntoPollable, Pollable};
use server_fx::http::router::{HandleRouteResult, Parameters, Router, RouteHandler};

pub(crate) struct SimpleHtmlRouteHandler {
    base_path: PathBuf,
}

impl SimpleHtmlRouteHandler {
    pub(crate) fn new<P: Into<PathBuf>>(base_path: P) -> SimpleHtmlRouteHandler {
        SimpleHtmlRouteHandler {
            base_path: base_path.into(),
        }
    }
}

fn mime_type_for_extension(ext: Option<&OsStr>) -> Option<&'static str> {
    static MIME_MAP: &'static [(&'static str, &'static str)] = &[
        ("html", "text/html"),
        ("css", "text/css"),
        ("js", "text/javascript"),
    ];

    ext.and_then(|ext| MIME_MAP.iter().position(|&(e, _)| e == ext)
            .map(|n| MIME_MAP[n].1))
}

fn debug_request(r: &types::Request) {
    write!(io::stdout(), "{} {} {}\r\n", 
           r.method(),
           r.path(),
           r.version())
        .expect("Couldn't write to STDOUT");

    for (name, value) in r.headers() {
        write!(io::stdout(), "{}: {}\r\n", name, value)
            .expect("Couldn't write to STDOUT");
    }

    writeln!(io::stdout(), "")
        .expect("Couldn't write to STDOUT");

}

impl RouteHandler for SimpleHtmlRouteHandler {
    fn handle(&self, 
              request: types::Request, 
              params: &Parameters) 
        -> types::Response 
    {
        let abs_path = self.base_path.join(&request.path()[1..]);
        let mime = mime_type_for_extension(abs_path.extension());

        if !abs_path.exists() || mime.is_none() {
            let mut response = types::ResponseBuilder::new(404, "Not found")
                .build();

            response.add_header("Connection", "close");
            return response;
        }

        let mut buf = vec![];
        ::std::fs::File::open(&abs_path)
            .expect(&format!("Cannot find '{}'", abs_path.to_str().unwrap()))
            .read_to_end(&mut buf)
            .unwrap();

        let mut response = types::ResponseBuilder::new(200, "OK")
            .build_with_stream(buf);
        response.add_header("Content-Type", mime.unwrap());

        response
    }
}


struct HandlerError;

pub(super) struct HttpServer(pub(super) Router);

impl Handler for HttpServer {
    type Request = types::Request;
    type Response = (types::Response, types::BodyChunk);
    type Error = io::Error;
    type Pollable = Box<Pollable<Item=Self::Response, Error=io::Error>>;

    fn handle(&self, request: Self::Request) -> Self::Pollable {

        debug_request(&request);

        let resp = match self.0.route(request) {
            HandleRouteResult::NotHandled(_) => {
                let mut response = types::ResponseBuilder::new(404, "Not Found")
                    .build();

                response.add_header("Connection", "close");
                response
            },
            HandleRouteResult::Handled(r) => r,
        };

        Box::new(
            resp.into_pollable()
                .map_err(|_| io::Error::from(io::ErrorKind::Other))
        )
    }
}

