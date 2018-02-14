use pollable::Pollable;

pub trait Handler {

    type Request;
    type Response;
    type Error;
    type Pollable: Pollable<Item=Self::Response, Error=Self::Error>;

    fn handle(&self, request: Self::Request) -> Self::Pollable;
}
