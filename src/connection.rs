use std::sync::Arc;

use handler::Handler;
use pollable::{IntoPollable, Pollable};
use result::PollResult;
use sink::{SendOne, Sink};

pub enum Connection<H, S> where
    H: Handler,
    S: Pollable<Item=H::Request> + Sink<Item=H::Response> + 'static
{
    Reading(S, Arc<H>),
    Handling(S, Arc<H>, <H::Pollable as IntoPollable>::Pollable),
    Writing(SendOne<S, H::Response>, Arc<H>),
    Done,
}

impl<H, S> Connection<H, S> where
    H: Handler,
    S: Pollable<Item=H::Request> + Sink<Item=H::Response> + 'static
{
    pub fn new(s: S, handler: Arc<H>) -> Connection<H, S> {
        Connection::Reading(s, handler)
    }
}

impl<H, S> Pollable for Connection<H, S> where 
    H: Handler,
    S: Pollable<Item=H::Request> + Sink<Item=H::Response> + 'static,
    H::Error: From<<S as Pollable>::Error>,
    H::Error: From<<S as Sink>::Error>,
{
    type Item = ();
    type Error = H::Error; //<S as Sink>::Error;

    fn poll(&mut self) -> Result<PollResult<Self::Item>, Self::Error> {
        use std::mem;

        let next = match mem::replace(self, Connection::Done) {
            Connection::Reading(mut stream, handler) => 
                match stream.poll()? {
                    PollResult::NotReady => 
                        Connection::Reading(stream, handler),
                    PollResult::Ready(request) => {
                        let pollable = handler.handle(request)
                            .into_pollable();
                        Connection::Handling(stream, handler, pollable)
                    },
                },
            Connection::Handling(s, h, mut pollable) => 
                match pollable.poll()? {
                    PollResult::NotReady => 
                        Connection::Handling(s, h, pollable),
                    PollResult::Ready(response) => 
                        Connection::Writing(s.send_one(response), h),
                },
            Connection::Writing(mut sink, h) => 
                match sink.poll()? {
                    PollResult::Ready(_) => Connection::Reading(sink.into_inner(), h), //return Ok(PollResult::Ready(())),
                    PollResult::NotReady => Connection::Writing(sink, h),
                },
            Connection::Done => panic!("Poll called on finished result"),
        };

        *self = next;
        Ok(PollResult::NotReady)
    }
}
