#[macro_export]
macro_rules! try_poll_io {
    ($e:expr) => {{
        match $e {
            Ok(n) => n,
            Err(ref e) 
                if e.kind() == ::std::io::ErrorKind::WouldBlock =>
                    return Ok(PollResult::NotReady),
            Err(e) => return Err(e.into()),
        }
    }}
}

mod server;
mod bind_transport;
mod handler;
mod pollable;
mod codec;
mod framed;
mod sink;
mod join;
mod result;
mod twist;
mod http;

fn main() {
    println!("Hello, world!");
}
