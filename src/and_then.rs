use pollable::Pollable;
use result::PollResult;

pub enum AndThen<L, F, R> {
    First(L, F),
    Second(R),
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
        Ok(PollResult::NotReady)
    }
}
