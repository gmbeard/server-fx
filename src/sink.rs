use result::PollResult;

pub enum SinkResult<T> {
    Ready,
    NotReady(T),
}

pub trait Sink {
    type Item;
    type Error;

    fn start_send(&mut self, item: Self::Item) -> Result<SinkResult<Self::Item>, Self::Error>;

    fn poll_complete(&mut self) -> Result<PollResult<()>, Self::Error>;
}
