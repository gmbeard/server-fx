use pollable::Pollable;
use result::PollResult;

pub struct MapErr<L, F>(L, Option<F>);

impl<L, F> MapErr<L, F> {
    pub fn new(l: L, f: F) -> MapErr<L, F> {
        MapErr(l, Some(f))
    }
}

impl<L, F, E> Pollable for MapErr<L, F> where
    L: Pollable,
    F: FnOnce(L::Error) -> E,
{
    type Item = L::Item;
    type Error = E;

    fn poll(&mut self) -> Result<PollResult<Self::Item>, Self::Error> {
        match self.0.poll() {
            Ok(r) => Ok(r),
            Err(e) => match self.1.take() {
                Some(f) => Err( f(e) ),
                None => panic!("Poll called on finished result"),
            }
        }
    }
}

