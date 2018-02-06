use pollable::Pollable;
use result::PollResult;

pub enum Join<L: Pollable, R: Pollable> {
    Neither(L, R),
    Left(L::Item, R),
    Right(L, R::Item),
    Done,
}

impl<L: Pollable, R: Pollable> Join<L, R> {
    pub fn new(left: L, right: R) -> Join<L, R> {
        Join::Neither(left, right)
    }
}

impl<L, R> Pollable for Join<L, R>
    where L: Pollable,
          R: Pollable,
          R::Error: From<L::Error>,
{
    type Item = (L::Item, R::Item);
    type Error = R::Error;

    fn poll(&mut self) -> Result<PollResult<Self::Item>, Self::Error> {
        use std::mem;

        let next = match mem::replace(self, Join::Done) {
            Join::Neither(mut left, mut right) => match (left.poll()?, right.poll()?) {
                (PollResult::Ready(lr), PollResult::Ready(rr)) => return Ok(PollResult::Ready((lr, rr))),
                (PollResult::Ready(lr), _) => Join::Left(lr, right),
                (_, PollResult::Ready(rr)) => Join::Right(left, rr),
                _ => Join::Neither(left, right),
            },
            Join::Left(lr, mut right) => match right.poll()? {
                PollResult::Ready(rr) => return Ok(PollResult::Ready((lr, rr))),
                _ => Join::Left(lr, right),
            },
            Join::Right(mut left, rr) => match left.poll()? {
                PollResult::Ready(lr) => return Ok(PollResult::Ready((lr, rr))),
                _ => Join::Right(left, rr),
            },
            Join::Done => panic!("Poll called on finished result"),
        };

        *self = next;

        Ok(PollResult::NotReady)
    }
}

#[cfg(test)]
mod pollable_should {
    use super::*;

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
