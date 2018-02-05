#[cfg_attr(test, derive(Debug, PartialEq))]
pub enum PollResult<T> {
    NotReady,
    Ready(T),
}

pub trait Pollable {
    type Item;
    type Error;

    fn poll(&mut self) -> Result<PollResult<Self::Item>, Self::Error>;

    fn fuse_result(self) -> FuseResult<Self>
        where Self: Sized
    {
        FuseResult::Polling(self)
    }

    fn join<R>(self, other: R) -> Join<Self, R>
        where R: Pollable<Item=Self::Item, Error=Self::Error>,
              Self: Sized,
    {
        Join::new(self, other)
    }
}

pub struct Join<L, R>(FuseResult<L>, FuseResult<R>)
    where L: Pollable,
          R: Pollable;

impl<L, R> Join<L, R> 
    where L: Pollable,
          R: Pollable,
{
    pub fn new(left: L, right: R) -> Join<L, R> {
        Join(left.fuse_result(), right.fuse_result())
    }
}

impl<L, R> Pollable for Join<L, R>
    where L: Pollable,
          R: Pollable,
          L::Error: From<R::Error>,
{
    type Item = (L::Item, R::Item);
    type Error = L::Error;

    fn poll(&mut self) -> Result<PollResult<Self::Item>, Self::Error> {
        match (self.0.poll(), self.1.poll()) {
            (Ok(PollResult::Ready(_)), Ok(PollResult::Ready(_))) => 
                Ok(PollResult::Ready((self.0.take().ok().unwrap(), self.1.take().ok().unwrap()))),
            (Err(_), _) => Err(self.0.take().err().unwrap()),
            (_, Err(_)) => Err(self.1.take().err().unwrap().into()),
            _ => Ok(PollResult::NotReady),
        }
    }
}

pub enum FuseResult<P: Pollable> {
    Polling(P),
    Finished(Result<P::Item, P::Error>),
    Empty,
}

impl<P: Pollable> FuseResult<P> {
    pub fn take(&mut self) -> Result<P::Item, P::Error> {
        use std::mem;

        if let FuseResult::Finished(r) = mem::replace(self, FuseResult::Empty) {
            r
        }
        else {
            panic!("Cannot incomplete FuseResult in current state")
        }
    }
}

impl<P: Pollable> Pollable for FuseResult<P> {

    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Result<PollResult<Self::Item>, Self::Error> {
        loop {
            let result = match *self {
                FuseResult::Polling(ref mut p) => {
                    match p.poll() {
                        Ok(PollResult::Ready(r)) => Ok(r),
                        Err(e) => Err(e),
                        _ => return Ok(PollResult::NotReady),
                    }
                },
                FuseResult::Finished(ref r) => match *r {
                    Ok(_) => return Ok(PollResult::Ready(())),
                    Err(_) => return Err(()),
                },
                FuseResult::Empty => panic!("FuseResult has already been taken"),
            };

            *self = FuseResult::Finished(result);
        }
    }
}

#[cfg(test)]
mod pollable_should {
    use super::*;

    #[test]
    fn fuse_result() {
        struct FuseTest;

        impl Pollable for FuseTest {
            type Item = usize;
            type Error = ();

            fn poll(&mut self) -> Result<PollResult<Self::Item>, Self::Error> {
                Ok(PollResult::Ready(42))
            }
        }

        let mut fuse_test = FuseTest.fuse_result();

        assert_eq!(Ok(PollResult::Ready(())), fuse_test.poll());
        assert_eq!(Ok(PollResult::Ready(())), fuse_test.poll());
        assert_eq!(Ok(PollResult::Ready(())), fuse_test.poll());
        assert_eq!(Ok(PollResult::Ready(())), fuse_test.poll());

        assert_eq!(Ok(42), fuse_test.take());
    }

    #[test]
    fn join() {
        struct YieldAfter(usize);

        impl Pollable for YieldAfter {
            type Item = usize;
            type Error = ();

            fn poll(&mut self) -> Result<PollResult<Self::Item>, Self::Error> {
                if self.0 == 0 {
                    return Ok(PollResult::Ready(42));
                }

                self.0 -= 1;

                Ok(PollResult::NotReady)
            }
        }

        let mut join = YieldAfter(0).join(YieldAfter(4));

        assert_eq!(Ok(PollResult::NotReady), join.poll());
        assert_eq!(Ok(PollResult::NotReady), join.poll());
        assert_eq!(Ok(PollResult::NotReady), join.poll());
        assert_eq!(Ok(PollResult::NotReady), join.poll());
        assert_eq!(Ok(PollResult::Ready((42, 42))), join.poll());
    }
}
