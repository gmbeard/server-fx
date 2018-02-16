use std::net::{self, ToSocketAddrs};
use std::io;
use std::sync::Arc;

use bind_transport::BindTransport;
use handler::Handler;
use pollable::{IntoPollable, Pollable};
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
        <P::Transport as Sink>::Error: From<<P::Result as IntoPollable>::Error>,
        <P::Transport as Sink>::Error: From<<P::Transport as Pollable>::Error>,
        <P::Transport as Sink>::Error: From<<H::Pollable as IntoPollable>::Error>,
        <P::Transport as Sink>::Error: ::std::fmt::Debug,
    {
        let listener = net::TcpListener::bind(s)?;
        let handler = Arc::new(f());

        for stream in listener.incoming() {

            let handler = handler.clone();
            let mut conn =  self.proto.bind_transport(stream?)
                .into_pollable()
                .and_then(move |transport| Connection::new(transport, handler));

            loop {
                match conn.poll() {
                    Ok(PollResult::Ready(_)) => break,
                    Err(e) => panic!("Error polling Connection: {:?}", e),
                    _ => {},
                }

                ::std::thread::yield_now();
            }
        }

        Ok(())
    }
}
