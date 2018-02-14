use join::Join;
use and_then::AndThen;
use result::PollResult;

pub trait Pollable {
    type Item;
    type Error;

    fn poll(&mut self) -> Result<PollResult<Self::Item>, Self::Error>;

    fn join<R>(self, other: R) -> Join<Self, R> where 
        R: Pollable,
        R::Error: From<Self::Error>,
        Self: Sized,
    {
        Join::new(self, other)
    }

    fn and_then<F, R>(self, f: F) -> AndThen<Self, F, R> where
        F: FnOnce(Self::Item) -> R,
        R: Pollable,
        R::Error: From<Self::Error>,
//        Self::Error: From<R::Error>,
        Self: Sized,
    {
        AndThen::new(self, f)
    }
}

pub trait IntoPollable {
    type Item;
    type Error;
    type Pollable: Pollable<Item=Self::Item, Error=Self::Error>;

    fn into_pollable(self) -> Self::Pollable;
}

impl<T, E> IntoPollable for Result<T, E> {
    type Item = T;
    type Error = E;
    type Pollable = PollableResult<T, E>;

    fn into_pollable(self) -> Self::Pollable {
        match self {
            Ok(value) => PollableResult::Ok(Some(value)),
            Err(error) => PollableResult::Err(Some(error)),
        }
    }
}

pub enum PollableResult<T, E> {
    Ok(Option<T>),
    Err(Option<E>),
}

impl<T, E> Pollable for PollableResult<T, E> {
    type Item = T;
    type Error = E;

    fn poll(&mut self) -> Result<PollResult<Self::Item>, Self::Error> {
        match *self {
            PollableResult::Ok(ref mut t) => match t.take() {
                Some(value) => Ok(PollResult::Ready(value)),
                None => panic!("Poll called on finished result"),
            },
            PollableResult::Err(ref mut e) => match e.take() {
                Some(error) => Err(error),
                None => panic!("Poll called on finished result"),
            }
        }
    }
}

