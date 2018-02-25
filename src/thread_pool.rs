use std::sync::Arc;
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::thread::{JoinHandle, spawn};
use std::marker::PhantomData;
use std::net;

use handler::Handler;
use bind_transport::BindTransport;
use result::PollResult;
use pollable::{IntoPollable, Pollable};
use sink::Sink;
use connection::Connection;

pub struct ThreadPool<P, H> {
    threads: Vec<JoinHandle<()>>,
    senders: Vec<Sender<net::TcpStream>>,
    last_thread: usize,
    _marker: PhantomData<(P, H)>,
}

impl<P, H> ThreadPool<P, H> where
    P: BindTransport<net::TcpStream> + Send + Sync + 'static,
    H: Handler<Request=P::Request, Response=P::Response> + Send + Sync + 'static,
    H::Error: From<<P::Transport as Sink>::Error>,
    H::Error: From<<P::Transport as Pollable>::Error>,
    H::Error: From<<P::Result as IntoPollable>::Error>,
    H::Error: ::std::fmt::Debug,
{
    pub fn new(num_threads: usize, proto: Arc<P>, handler: Arc<H>) 
        -> ThreadPool<P, H>
    {
        let mut threads = Vec::with_capacity(num_threads);
        let mut senders = Vec::with_capacity(num_threads);

        for _ in 0..num_threads {
            let (sender, receiver) = channel();
            let proto = proto.clone();
            let handler = handler.clone();
            let t = spawn(move || connection_proc(proto, handler, receiver));

            threads.push(t);
            senders.push(sender);
        }

        ThreadPool {
            threads: threads,
            senders: senders,
            last_thread: 0,
            _marker: PhantomData,
        }
    }

    pub fn queue(&mut self, stream: net::TcpStream) {
        self.senders[self.last_thread] .send(stream)
            .expect("The connection thread has died!");
        self.last_thread += 1;
        self.last_thread %= self.threads.len();
    }
}

fn connection_proc<P, H>(proto: Arc<P>, 
                         handler: Arc<H>, 
                         recv: Receiver<net::TcpStream>) 
    where
        P: BindTransport<net::TcpStream>, 
        H: Handler<Request=P::Request, Response=P::Response>,
        H::Error: From<<P::Transport as Sink>::Error>,
        H::Error: From<<P::Transport as Pollable>::Error>,
        H::Error: From<<P::Result as IntoPollable>::Error>,
        H::Error: ::std::fmt::Debug,
{
    let mut connections = vec![];

    loop {
        let msg = {
            if connections.len() == 0 {
                match recv.recv() {
                    Ok(s) => Some(s),
                    Err(_) => return,
                }
            }
            else {
                match recv.try_recv() {
                    Ok(s) => Some(s),
                    Err(TryRecvError::Empty) => None,
                    _ => return,
                }
            }
        };

        msg.map(|s| {
            let handler = handler.clone();
            let conn = proto.bind_transport(s)
                .into_pollable()
                .and_then(move |transport| Connection::new(transport, handler));

            connections.push(Some(conn));
        });

        pump_connections(&mut connections);
    }
}

fn pump_connections<P: Pollable>(connections: &mut Vec<Option<P>>) {

    for c in connections.iter_mut() {
        let mut current = c.take()
            .expect("There are no connections waiting to be polled!");

        if let Ok(PollResult::NotReady) =  current.poll() {
            *c = Some(current);
        }
    }

    let mut n = connections.len();
    while n > 0 {
        n -= 1;
        if connections[n].is_none() {
            connections.swap_remove(n);
        }
    }
}

