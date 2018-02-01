use std::io;
use pollable::Pollable;

pub trait BindTransport<S: io::Read> {
    type Request;
    type Response;
    type Transport: Pollable<Item=Self::Request>;

    fn bind_transport(&self, s: S) -> Self::Transport;
}
