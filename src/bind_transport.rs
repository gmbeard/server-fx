use std::io;
use pollable::{IntoPollable, Pollable};
use sink::Sink;

pub trait BindTransport<S> where
    S: io::Read + io::Write + 'static
{
    type Request;
    type Response;
    type Transport: Pollable<Item=Self::Request> + Sink<Item=Self::Response> + 'static;
    type Result: IntoPollable<Item=Self::Transport>;

    fn bind_transport(&self, s: S) -> Self::Result;
}
