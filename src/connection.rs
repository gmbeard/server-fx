use std::sync::Arc;

use handler::Handler;
use pollable::{IntoPollable, Pollable};
use result::PollResult;
use sink::{SendOne, Sink, SinkResult};

pub enum Connection<H, S> where
    H: Handler,
    S: Pollable<Item=H::Request> + Sink<Item=H::Response> + 'static
{
    Reading(S, Arc<H>),
    Handling(S, <H::Pollable as IntoPollable>::Pollable),
    Writing(SendOne<S, H::Response>),
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
    <S as Sink>::Error: From<<S as Pollable>::Error>,
    <S as Sink>::Error: From<<H::Pollable as IntoPollable>::Error>,
{
    type Item = ();
    type Error = <S as Sink>::Error;

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
                        Connection::Handling(stream, pollable)
                    },
                },
            Connection::Handling(s, mut pollable) => 
                match pollable.poll()? {
                    PollResult::NotReady => 
                        Connection::Handling(s, pollable),
                    PollResult::Ready(response) => 
                        Connection::Writing(s.send_one(response)),
                },
            Connection::Writing(mut sink) => 
                match sink.poll()? {
                    PollResult::Ready(_) => return Ok(PollResult::Ready(())),
                    PollResult::NotReady => Connection::Writing(sink),
                },
            Connection::Done => panic!("Poll called on finished result"),
        };

        *self = next;
        Ok(PollResult::NotReady)
    }
}
