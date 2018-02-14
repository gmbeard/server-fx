use std::net::{self, ToSocketAddrs};
use std::io;
use std::sync::Arc;

use bind_transport::BindTransport;
use handler::Handler;
use pollable::Pollable;
use sink::Sink;
use result::PollResult;
use connection::Connection;

pub struct TcpServer<P> {
    proto: Arc<P>,
}

impl<P> TcpServer<P> 
    where P: BindTransport<net::TcpStream>
{
    pub fn new(proto: P) -> TcpServer<P> {
        TcpServer { 
            proto: Arc::new(proto) 
        }
    }

    pub fn serve<S, F, H>(self, s: S, f: F) -> io::Result<()> where 
        S: ToSocketAddrs,
        F: Fn() -> H,
        H: Handler<Request=P::Request, Response=P::Response>,
        <P::Transport as Sink>::Error: From<<P::Transport as Pollable>::Error>,
        <P::Transport as Sink>::Error: From<<H::Pollable as Pollable>::Error>,
    {
        let listener = net::TcpListener::bind(s)?;
        let handler = Arc::new(f());

        for stream in listener.incoming() {

            let mut conn = Connection::new(
                self.proto.bind_transport(stream?).unwrap(),
                handler.clone()
            );

            loop {
                match conn.poll() {
                    Ok(PollResult::Ready(_)) => break,
                    Err(_) => panic!("Error polling Connection"),
                    _ => continue,
                }
            }
        }

        Ok(())
    }
}
