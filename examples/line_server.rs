extern crate server_fx;

use std::io::{self, Read, Write};

use server_fx::bind_transport::BindTransport;
use server_fx::framed::Framed;
use server_fx::codec::{Decode, Encode};
use server_fx::server::TcpServer;
use server_fx::pollable::{IntoPollable, Pollable, PollableResult};
use server_fx::handler::Handler;

struct LineCodec;

impl Decode for LineCodec {
    type Item = Vec<u8>;

    fn decode(&self, buffer: &mut Vec<u8>) -> Option<Self::Item> {
        if let Some(pos) = buffer.iter()
            .position(|v| *v == b'\r' || *v == b'\n')
        {
            let v = buffer.drain(..pos).collect::<Vec<_>>();
            while buffer.get(0).take()
                .map(|&b| b == b'\n')
                .unwrap_or(false)
            {
                buffer.drain(..1);
            }
            return Some(v);
        }
        None
    }
}

impl Encode for LineCodec {
    type Item = Vec<u8>;

    fn encode(&self, item: Self::Item, buffer: &mut Vec<u8>) {
        buffer.extend(&item);
    }
}

struct LineProto;

impl<Io> BindTransport<Io> for LineProto where
    Io: Read + Write + 'static
{
    type Request = Vec<u8>;
    type Response = Vec<u8>;
    type Transport = Framed<Io, LineCodec>;
    type Result = Result<Self::Transport, io::Error>;

    fn bind_transport(&self, io: Io) -> Self::Result {
        Ok(Framed::new(io, LineCodec))
    }
}

struct Server;

impl Handler for Server {
    type Request = Vec<u8>;
    type Response = Vec<u8>;
    type Error = io::Error;
    type Pollable = PollableResult<Self::Response, Self::Error>;

    fn handle(&self, request: Self::Request) -> Self::Pollable {
        Ok(request).into_pollable()
    }
}

fn main() {
    TcpServer::new(LineProto)
        .serve("127.0.0.1:5051", || Server)
        .unwrap();
}
