use std::net::{self, ToSocketAddrs};
use std::io;

use bind_transport::BindTransport;
use handler::Handler;
use pollable::{Pollable, PollResult};

struct TcpServer<P> {
    proto: P
}

impl<P> TcpServer<P> 
    where P: BindTransport<net::TcpStream>
{
    fn new(proto: P) -> TcpServer<P> {
        TcpServer { proto: proto }
    }

    fn serve<S, F, H>(self, s: S, f: F) -> io::Result<()> 
        where S: ToSocketAddrs,
              F: Fn() -> H,
              H: Handler<Request=P::Request>
    {
        let listener = net::TcpListener::bind(s)?;

        for stream in listener.incoming() {
            let mut transport = self.proto.bind_transport(stream?);

            let request = loop {
                match transport.poll() {
                    Ok(PollResult::Ready(request)) => break Ok(request),
                    Err(e) => break Err(e),
                    _ => continue,
                }
            };

            if let Ok(request) = request {
                let handler = f();
                let mut response = handler.handle(request);

                loop {
                    if let Ok(_) = response.poll() {
                        break;
                    }
                }
            }
        }

        Ok(())
    }
}
