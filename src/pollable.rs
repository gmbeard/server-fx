use join::Join;
use and_then::AndThen;
use result::PollResult;
use map_err::MapErr;

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
        Self: Sized,
    {
        AndThen::new(self, f)
    }

    fn map_err<F, E>(self, f: F) -> MapErr<Self, F> where
        F: FnOnce(Self::Error) -> E,
        Self: Sized,
    {
        MapErr::new(self, f)
    }
}

impl<P: Pollable + ?Sized> Pollable for Box<P> {
    type Item = P::Item;
    type Error = P::Error;

    fn poll(&mut self) -> Result<PollResult<Self::Item>, Self::Error> {
        (&mut **self).poll()
    }
}

pub trait IntoPollable {
    type Item;
    type Error;
    type Pollable: Pollable<Item=Self::Item, Error=Self::Error>;

    fn into_pollable(self) -> Self::Pollable;
}

impl<P: Pollable> IntoPollable for P {
    type Item = P::Item;
    type Error = P::Error;
    type Pollable = P;

    fn into_pollable(self) -> Self::Pollable {
        self
    }
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

