use std::rc::Rc;
use std::io::{self, Read, Write};
use std::fmt::Debug;

use pollable::Pollable;
use result::PollResult;
use join::Join;

enum TransferState {
    Reading,
    Writing(usize),
}

struct Transfer<S, D> {
    source: Rc<S>,
    destination: Rc<D>,
    buffer: Vec<u8>,
    state: TransferState,
    transferred: usize,
}

const BUFFER_SIZE: usize = 1024*8;

impl<S, D> Transfer<S, D> {
    fn new(source: Rc<S>, destination: Rc<D>) -> Transfer<S, D> {
        Transfer {
            source: source,
            destination: destination,
            buffer: vec![0_u8; BUFFER_SIZE],
            state: TransferState::Reading,
            transferred: 0,
        }
    }
}

impl<S, D> Transfer<S, D> {
    fn into_inner(self) -> (Rc<S>, Rc<D>) {
        (self.source, self.destination)
    }
}

impl<S, D> Pollable for Transfer<S, D>
    where for <'a> &'a S: Read,
          for <'a> &'a D: Write,
{
    type Item = usize;
    type Error = io::Error;

    fn poll(&mut self) -> Result<PollResult<Self::Item>, Self::Error> {
        loop {
            let next = match self.state {
                TransferState::Reading => {
                    let n = try_poll_io!((&*self.source).read(&mut self.buffer));
                    if 0 == n {
                        return Ok(PollResult::Ready(self.transferred));
                    }

                    TransferState::Writing(n)
                },
                TransferState::Writing(remaining) => {
                    let result = (&*self.destination).write(&self.buffer[..remaining]);
                    
                    match try_poll_io!(result) {
                        0 => return Ok(PollResult::Ready(self.transferred)),
                        n if n == remaining => {
                            self.transferred += remaining;
                            TransferState::Reading
                        },
                        n => {
                            self.transferred += n;
                            TransferState::Writing(remaining - n)
                        },
                    }
                },
            };

            self.state = next;
        }
    }
}

type Twist<S, D> = Join<Transfer<S, D>, Transfer<D, S>>;

pub struct Twister<S, D>(Twist<S, D>)
    where for <'a> &'a S: Read + Write,
          for <'a> &'a D: Read + Write;

impl<S, D> Twister<S, D>
    where for <'a> &'a S: Read + Write,
          for <'a> &'a D: Read + Write,
{
    pub fn new(source: S, destination: D) -> Twister<S, D> {
        let source = Rc::new(source);
        let destination = Rc::new(destination);

        let inner = 
            Transfer::new(source.clone(), destination.clone())
                .join(Transfer::new(destination, source));

        Twister(inner)
    }
}

impl<S, D> Twister<S, D>
    where for <'a> &'a S: Read + Write,
          for <'a> &'a D: Read + Write,
          S: Debug,
          D: Debug,
{
    pub fn into_inner(self) -> (S, D) {
        let (src_transfer, dest_transfer) = self.0.into_inner();
        let (src, _) = src_transfer.into_inner();
        let (dst, _) = dest_transfer.into_inner();
        (Rc::try_unwrap(src).unwrap(), Rc::try_unwrap(dst).unwrap())
    }
}

impl<S, D> Pollable for Twister<S, D>
    where for <'a> &'a S: Read + Write,
          for <'a> &'a D: Read + Write,
{
    type Item = (usize, usize);
    type Error = io::Error;

    fn poll(&mut self) -> Result<PollResult<Self::Item>, Self::Error> {
        self.0.poll()
    }
}

#[cfg(test)]
mod twister_should {
    use super::*;
    use std::cell::{Ref, RefCell};
    use std::io::{self, Cursor, Read, Write};

    // This type wraps a `Read` type and simulates
    // bytes arriving after `ready_every` calls. It
    // also trickles `read`s one byte at a time.
    #[derive(Debug)]
    struct Trickle<R> {
        inner: R,
        call_count: usize,
        ready_every: usize,
    }

    impl<R> Trickle<R> {
        fn new(inner: R, ready_every: usize) -> Trickle<R> {
            Trickle {
                inner: inner,
                call_count: 0,
                ready_every: ready_every,
            }
        }
    }

    impl<R: Read> Read for Trickle<R> {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            self.call_count += 1;
            if 0 == ((self.call_count - 1) % self.ready_every) {
                self.inner.read(&mut buffer[..1])
            }
            else {
                Err(io::ErrorKind::WouldBlock.into())
            }
        }
    }

    // `Half` must wrap its `Read` and `Write` implementations
    // in `RefCell`s so that we can read and write to it
    // using a shared alias (I.E. `&self`, instead of `&mut self`).
    // This is a requirement of `Twister`
    #[derive(Debug)]
    struct Half {
        output: RefCell<Trickle<Cursor<Vec<u8>>>>,
        input: RefCell<Cursor<Vec<u8>>>,
    }

    impl Half {
        fn new(initial_content: &[u8], ready_every: usize) -> Half {
            Half {
                output: RefCell::new(Trickle::new(Cursor::new(initial_content.to_vec()), ready_every)),
                input: RefCell::new(Cursor::new(vec![])),
            }
        }

        fn write_buffer(&self) -> Ref<Cursor<Vec<u8>>> {
            self.input.borrow()
        }
    }

    impl<'a> Read for &'a Half {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            self.output.borrow_mut().read(buffer)
        }
    }

    impl<'a> Write for &'a Half {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            self.input.borrow_mut().write(buffer)
        }

        fn flush(&mut self) -> io::Result<()> {
            self.input.borrow_mut().flush()
        }
    }

    #[test]
    fn copy_both_halves() {
        let first_content = b"Hello, from first half";
        let second_content = b"Hello, from second half";
        let first_half = Half::new(first_content, 1);
        let second_half = Half::new(second_content, 7);

        let mut twister = Twister::new(first_half, second_half);
        let value = loop {
            if let PollResult::Ready(v) = twister.poll().unwrap() {
                break v;
            }
        };

        let (first_half, second_half) = twister.into_inner();

        assert_eq!(
            (first_content.len(), second_content.len()),
            value
        );

        assert_eq!(
            b"Hello, from second half",
            &(*first_half.write_buffer()).get_ref()[..]
        );

        assert_eq!(
            b"Hello, from first half",
            &(*second_half.write_buffer()).get_ref()[..]
        );
    }
}

