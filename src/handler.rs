use pollable::IntoPollable;

pub trait Handler {

    type Request;
    type Response;
    type Error;
    type Pollable: IntoPollable<Item=Self::Response, Error=Self::Error>;

    fn handle(&self, request: Self::Request) -> Self::Pollable;
}
