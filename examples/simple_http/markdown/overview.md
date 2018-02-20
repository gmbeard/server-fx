Overview
===

Asyncronous operations are implemented using *Server Fx*'s `Pollable`
trait.

```rust
pub type Poll<T, E> = Result<PollResult<T>, E>;

pub trait Pollable {
    type Item;
    type Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error>;
    ...
}
```

As long as an operation provides an implementation of the `Pollable::poll()` 
function, *Server Fx* will take care of busy polling it operation to 
completion.

```rust
// A simple async operation...
struct SimpleAsync(usize);

impl Pollable for SimpleAsync {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if self.0 == 4 {
            return Ok(PollResult::Ready(()));
        }

        self.0 += 1;    
        Ok(PollResult::NotReady)
    }
}

let mut simple_async = SimpleAsync(1);
```

Have a look how to use *Server Fx* to implement a [HTTP server][1]

[1]: #http_server
