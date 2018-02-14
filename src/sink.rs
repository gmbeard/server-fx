use result::PollResult;
use pollable::Pollable;

pub enum SinkResult<T> {
    Ready,
    NotReady(T),
}

pub trait Sink {
    type Item;
    type Error;

    fn start_send(&mut self, item: Self::Item) -> Result<SinkResult<Self::Item>, Self::Error>;

    fn poll_complete(&mut self) -> Result<PollResult<()>, Self::Error>;

    fn send_one(self, item: Self::Item) -> SendOne<Self, Self::Item> where
        Self: Sized
    {
        SendOne::new(self, item)
    }
}

pub struct SendOne<S, I> {
    inner: S,
    value: Option<I>,
}

impl<S, I> SendOne<S, I> {
    pub fn new(inner: S, value: I) -> SendOne<S, I> {
        SendOne {
            inner: inner,
            value: Some(value),
        }
    }
}

impl<S, I> Pollable for SendOne<S, I>
    where S: Sink<Item=I> + 'static
{
    type Item = ();
    type Error = S::Error;

    fn poll(&mut self) -> Result<PollResult<Self::Item>, Self::Error> {
        loop {
            match self.value.take() {
                Some(value) => {
                    if let SinkResult::NotReady(value) =
                        self.inner.start_send(value)?
                    {
                        self.value = Some(value);
                        if let PollResult::NotReady =
                            self.inner.poll_complete()?
                        {
                            return Ok(PollResult::NotReady);
                        }
                    }
                },
                None => return self.inner.poll_complete(),
            }
        }

        Ok(PollResult::NotReady)
    }
}

