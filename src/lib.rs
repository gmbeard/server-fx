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

pub mod server;
pub mod bind_transport;
pub mod handler;
pub mod pollable;
pub mod codec;
pub mod framed;
pub mod sink;
pub mod join;
pub mod and_then;
pub mod result;
pub mod twist;
pub mod http;
pub mod connection;
pub mod map_err;
