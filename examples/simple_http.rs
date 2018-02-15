extern crate server_fx;

use std::io::{self, Read, Write};

use server_fx::handler::Handler;
use server_fx::http::types;
use server_fx::codec::{Decode, Encode};
use server_fx::bind_transport::BindTransport;
use server_fx::server::TcpServer;
use server_fx::framed::Framed;
use server_fx::pollable::{IntoPollable, PollableResult};

struct HttpServer;

macro_rules! quick_str {
    ($e: expr) => {
        ::std::str::from_utf8($e).unwrap()
    }
}

impl Handler for HttpServer {
    type Request = types::Request;
    type Response = Vec<u8>;
    type Error = io::Error;
    type Pollable = PollableResult<Self::Response, Self::Error>;

    fn handle(&self, _request: Self::Request) -> Self::Pollable {

        write!(io::stdout(), "<METHOD> {} {}\r\n", 
               quick_str!(_request.path()),
               quick_str!(_request.version()));
        for (name, value) in _request.headers() {
            write!(io::stdout(), "{}: {}\r\n", 
                   quick_str!(name), quick_str!(value));
        }
        writeln!(io::stdout(), "");

        static RESPONSE: &'static [u8] = 
            b"HTTP/1.1 302 Moved\r\n\
              Content-Length: 0\r\n\
              Location: /about.html\r\n\
              Connection: close\r\n\
              \r\n";

        Ok(RESPONSE.to_vec()).into_pollable()
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
    type Item = Vec<u8>;

    fn encode(&self, response: Self::Item, buffer: &mut Vec<u8>) {
        ::std::mem::replace(buffer, response);
    }
}

struct HttpProto;

impl<Io> BindTransport<Io> for HttpProto where
    Io: io::Read + io::Write + 'static
{
    type Request = types::Request;
    type Response = Vec<u8>;
    type Transport = Framed<Io, HttpCodec>;

    fn bind_transport(&self, io: Io) -> Result<Self::Transport, ()> {
        Ok(Framed::new(io, HttpCodec))
    }
}

fn main() {
    TcpServer::new(HttpProto)
        .serve("127.0.0.1:50001", || HttpServer);
}
