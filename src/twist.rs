use std::rc::Rc;
use std::io::{self, Read, Write};
use pollable::Pollable;
use result::PollResult;
use join::Join;
use std::net::TcpStream;

enum TransferState {
    Reading,
    Writing(usize),
    Done,
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

impl<S, D> Pollable for Transfer<S, D>
    where for <'a> &'a S: Read,
          for <'a> &'a D: Write,
{
    type Item = usize;
    type Error = io::Error;

    fn poll(&mut self) -> Result<PollResult<Self::Item>, Self::Error> {
        use std::mem;

        loop {
            match self.state {
                TransferState::Reading => {
                    match try_poll_io!((&*self.source).read(&mut self.buffer)) {
                        0 => return Ok(PollResult::Ready(self.transferred)),
                        n => self.state = TransferState::Writing(n),
                    }
                },
                TransferState::Writing(remaining) => {
                    match try_poll_io!((&*self.destination).write(&self.buffer[..remaining])) {
                        0 => return Ok(PollResult::Ready(self.transferred)),
                        n if n == remaining => {
                            self.transferred += remaining;
                            self.state = TransferState::Reading;
                        },
                        n => {
                            self.transferred += n;
                            self.state = TransferState::Writing(remaining - n);
                        },
                    }
                },
                TransferState::Done => panic!("Poll called on finished result"),
            }
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
    pub fn new(source: Rc<S>, destination: Rc<D>) -> Twister<S, D> {
        let inner = 
            Transfer::new(source.clone(), destination.clone())
                .join(Transfer::new(destination, source));

        Twister(inner)
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
        let first_half = Rc::new(Half::new(first_content, 1));
        let second_half = Rc::new(Half::new(second_content, 7));    

        let mut twister = Twister::new(first_half.clone(), second_half.clone());
        let value = loop {
            if let PollResult::Ready(v) = twister.poll().unwrap() {
                break v;
            }
        };

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

