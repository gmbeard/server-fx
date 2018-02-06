#[cfg_attr(test, derive(Debug, PartialEq))]
pub enum PollResult<T> {
    NotReady,
    Ready(T),
}
