extern crate server_fx;

use std::io::{self, Write};

use server_fx::handler::Handler;
use server_fx::http::types;
use server_fx::codec::{Decode, Encode};
use server_fx::bind_transport::BindTransport;
use server_fx::server::TcpServer;
use server_fx::framed::Framed;
use server_fx::pollable::{IntoPollable, PollableResult};

struct HttpServer;

macro_rules! str {
    ($e: expr) => {
        ::std::str::from_utf8($e).unwrap()
    }
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

impl Handler for HttpServer {
    type Request = types::Request;
    type Response = types::Response;
    type Error = io::Error;
    type Pollable = Result<Self::Response, Self::Error>;

    fn handle(&self, request: Self::Request) -> Self::Pollable {

        debug_request(&request);

        let mut response = types::ResponseBuilder::new(302, "Moved")
            .build();

        response.add_header("Content-Length", "0");
        response.add_header("Location", "/about.html");
        response.add_header("Connection", "close");

//        static RESPONSE: &'static [u8] = 
//            b"HTTP/1.1 302 Moved\r\n\
//              Content-Length: 0\r\n\
//              Location: /about.html\r\n\
//              Connection: close\r\n\
//              \r\n";

        Ok(response)
    }
}

struct HttpCodec;

impl Decode for HttpCodec {
    type Item = types::Request;

    fn decode(&self, buffer: &mut Vec<u8>) -> Option<Self::Item> {
        types::parse_request(buffer)        
    }
}

impl Encode for HttpCodec {
    type Item = types::Response;

    fn encode(&self, response: Self::Item, buffer: &mut Vec<u8>) {
        let mut s = format!("{} {} {}\r\n",
                        response.version(),
                        response.status_code(),
                        response.status_text());
        for (n, v) in response.headers() {
            s.push_str(format!("{}: {}\r\n", n, v).as_ref());
        }
        s.push_str(format!("\r\n").as_ref());

        buffer.extend(s.as_bytes());
    }
}

struct HttpProto;

impl<Io> BindTransport<Io> for HttpProto where
    Io: io::Read + io::Write + 'static
{
    type Request = types::Request;
    type Response = types::Response;
    type Transport = Framed<Io, HttpCodec>;
    type Result = Result<Self::Transport, io::Error>;

    fn bind_transport(&self, io: Io) -> Self::Result {
        Ok(Framed::new(io, HttpCodec))
    }
}

fn main() {
    TcpServer::new(HttpProto)
        .serve("127.0.0.1:5050", || HttpServer)
        .unwrap();
}
