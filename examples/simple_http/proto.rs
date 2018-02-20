use std::io::{self, Read, Write};

use server_fx::codec::{Decode, Encode};
use server_fx::http::types;
use server_fx::bind_transport::BindTransport;
use server_fx::framed::Framed;

pub(crate) struct HttpCodec;

impl Decode for HttpCodec {
    type Item = types::Request;

    fn decode(&self, buffer: &mut Vec<u8>) -> Option<Self::Item> {
        types::parse_request(buffer)
    }
}

impl Encode for HttpCodec {
    type Item = (types::Response, types::BodyChunk);

    fn encode(&self, response: Self::Item, buffer: &mut Vec<u8>) {
        let mut s = format!("{} {} {}\r\n",
                        response.0.version(),
                        response.0.status_code(),
                        response.0.status_text());
        for (n, v) in response.0.headers() {
            s.push_str(format!("{}: {}\r\n", n, v).as_ref());
        }
        s.push_str(format!("Content-Length: {}\r\n", response.1.len()).as_ref());
        s.push_str(format!("\r\n").as_ref());

        buffer.extend(s.as_bytes());
        buffer.extend(response.1);
    }
}

pub(crate) struct HttpProto;

impl<Io> BindTransport<Io> for HttpProto where
    Io: io::Read + io::Write + 'static
{
    type Request = types::Request;
    type Response = (types::Response, types::BodyChunk);
    type Transport = Framed<Io, HttpCodec>;
    type Result = Result<Self::Transport, io::Error>;

    fn bind_transport(&self, io: Io) -> Self::Result {
        Ok(Framed::new(io, HttpCodec))
    }
}

