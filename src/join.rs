use pollable::Pollable;
use result::PollResult;

enum JoinState<L, R> {
    Niether,
    Left(L),
    Right(R),
    Done
}

pub struct Join<L: Pollable, R: Pollable> {
    left: L,
    right: R,
    state: JoinState<L::Item, R::Item>,
//    Neither(L, R),
//    Left(L::Item, R),
//    Right(L, R::Item),
//    Done,
}

impl<L: Pollable, R: Pollable> Join<L, R> {
    pub fn new(left: L, right: R) -> Join<L, R> {
        Join {
            left: left,
            right: right,
            state: JoinState::Niether,
        }
//        Join::Neither(left, right)
    }

    pub fn into_inner(self) -> (L, R) {
        (self.left, self.right)
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

        let next = match mem::replace(&mut self.state, JoinState::Done) {
            JoinState::Niether => match (self.left.poll()?, self.right.poll()?) {
                (PollResult::Ready(lr), PollResult::Ready(rr)) => return Ok(PollResult::Ready((lr, rr))),
                (PollResult::Ready(lr), _) => JoinState::Left(lr),
                (_, PollResult::Ready(rr)) => JoinState::Right(rr),
                _ => JoinState::Niether,
            },
            JoinState::Left(lr) => match self.right.poll()? {
                PollResult::Ready(rr) => return Ok(PollResult::Ready((lr, rr))),
                _ => JoinState::Left(lr),
            },
            JoinState::Right(rr) => match self.left.poll()? {
                PollResult::Ready(lr) => return Ok(PollResult::Ready((lr, rr))),
                _ => JoinState::Right(rr),
            },
            JoinState::Done => panic!("Poll called on finished result"),
        };

        self.state = next;

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
