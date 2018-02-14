use std::io;
use pollable::Pollable;
use sink::Sink;

pub trait BindTransport<S: io::Read> {
    type Request;
    type Response;
    type Transport: Pollable<Item=Self::Request> + Sink<Item=Self::Response> + 'static;

    fn bind_transport(&self, s: S) -> Result<Self::Transport, ()>;
}
