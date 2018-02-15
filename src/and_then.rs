use std::mem;

use pollable::Pollable;
use result::PollResult;

pub enum AndThen<L, F, R> {
    First(L, F),
    Second(R),
    Done,
}

impl<L, F, R> AndThen<L, F, R> where
    L: Pollable,
    F: FnOnce(L::Item) -> R,
{
    pub fn new(left: L, f: F) -> AndThen<L, F, R> {
        AndThen::First(left, f)
    }
}

impl<L, F, R> Pollable for AndThen<L, F, R> where
    L: Pollable,
    F: FnOnce(L::Item) -> R,
    R: Pollable,
    R::Error: From<L::Error>,
{
    type Item = R::Item;
    type Error = R::Error;

    fn poll(&mut self) -> Result<PollResult<Self::Item>, Self::Error> {
        let result = match *self {
            AndThen::First(ref mut left, _) => match left.poll() {
                Ok(PollResult::Ready(value)) => Ok(value),
                Ok(PollResult::NotReady) => return Ok(PollResult::NotReady),
                Err(e) => Err(e),
            },
            AndThen::Second(ref mut right) => return right.poll(),
            AndThen::Done => panic!("Poll called on finished result"),
        };
        
        let next = match mem::replace(self, AndThen::Done) {
            AndThen::First(_, f) => {
                let left_value = result?;
                let mut right = f(left_value);
                if let PollResult::Ready(right_value) = right.poll()? {
                    return Ok(PollResult::Ready(right_value));
                }

                AndThen::Second(right)
            },
            _ => unreachable!(),
        };

        *self = next;
        Ok(PollResult::NotReady)
    }
}
