use std::io::{self, Read, Write};
use codec::{Decode, Encode};
use pollable::Pollable;
use sink::{Sink, SinkResult};
use result::PollResult;

type Poll<T, E> = Result<PollResult<T>, E>;
type StartSend<T, E> = Result<SinkResult<T>, E>;

pub struct Framed<S, D> {
    stream: S,
    decoder: D,
    buffer: Vec<u8>,
}

impl<S, D> Framed<S, D> {
    pub fn new(stream: S, codec: D) -> Framed<S, D> {
        Framed {
            stream: stream,
            decoder: codec,
            buffer: Vec::with_capacity(1024),
        }
    }
}

impl<S, D> Framed<S, D>
    where S: Read,
          D: Decode + Encode,
{
    pub fn into_stream(self) -> S {
        self.stream
    }
}

impl<S, D> Pollable for Framed<S, D>
    where S: Read,
          D: Decode,
{
    type Item = D::Item;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let mut buf = [0_u8; 256];

        loop {
            let bytes_read = match try_poll_io!(self.stream.read(&mut buf)) {
                0 => return Err(io::ErrorKind::UnexpectedEof.into()),
                n => n,
            };

            self.buffer.extend(&buf[..bytes_read]);

            if let Some(request) = self.decoder.decode(&mut self.buffer) {
                return Ok(PollResult::Ready(request));
            }
        }
    }
}

impl<S, E> Sink for Framed<S, E>
    where S: Write,
          E: Encode,
{
    type Item = E::Item;
    type Error = io::Error;

    fn start_send(&mut self, item: Self::Item) -> StartSend<Self::Item, Self::Error> {
        if self.buffer.len() != 0 {
            return Ok(SinkResult::NotReady(item));
        }
        self.decoder.encode(item, &mut self.buffer);
        Ok(SinkResult::Ready)
    }

    fn poll_complete(&mut self) -> Poll<(), Self::Error> {
        match try_poll_io!(self.stream.write(&self.buffer)) {
            0 => Ok(PollResult::Ready(())),
            n => {
                self.buffer.drain(..n);
                if self.buffer.len() == 0 {
                    Ok(PollResult::NotReady)
                }
                else {
                    Ok(PollResult::Ready(()))
                }
            }
        }
    }
}
