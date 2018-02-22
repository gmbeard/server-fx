use std::net::{self, ToSocketAddrs};
use std::io;
use std::sync::Arc;

use bind_transport::BindTransport;
use handler::Handler;
use pollable::{IntoPollable, Pollable};
use sink::Sink;
use thread_pool::ThreadPool;

const NUM_THREADS: usize = 4;

pub struct TcpServer<P> {
    proto: Arc<P>,
}

impl<P> TcpServer<P> 
    where P: BindTransport<net::TcpStream> + Send + Sync + 'static,
{
    pub fn new(proto: P) -> TcpServer<P> {
        TcpServer { 
            proto: Arc::new(proto) 
        }
    }

    pub fn serve<S, F, H>(self, s: S, f: F) -> io::Result<()> where 
        S: ToSocketAddrs,
        F: FnOnce() -> H,
        H: Handler<Request=P::Request, Response=P::Response> + Send + Sync + 'static,
        H::Error: From<<P::Transport as Sink>::Error>,
        H::Error: From<<P::Transport as Pollable>::Error>,
        H::Error: From<<P::Result as IntoPollable>::Error>,
        H::Error: ::std::fmt::Debug,
    {
        let listener = net::TcpListener::bind(s)?;
        let handler = Arc::new(f());
        let mut pool = ThreadPool::new(NUM_THREADS, 
                                       self.proto.clone(), 
                                       handler.clone());

        for stream in listener.incoming() {
            pool.queue(stream?);
        }

        Ok(())
    }
}
