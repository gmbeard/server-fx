use join::Join;
use result::PollResult;

pub trait Pollable {
    type Item;
    type Error;

    fn poll(&mut self) -> Result<PollResult<Self::Item>, Self::Error>;

    fn join<R>(self, other: R) -> Join<Self, R>
        where R: Pollable,
              R::Error: From<Self::Error>,
              Self: Sized,
    {
        Join::new(self, other)
    }
}
