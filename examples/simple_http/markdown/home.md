Server Fx
===

**Server-Fx** is a framework for developing asynchronous network
applications. It takes inspiration from the [Futures][1] and [Tokio][2] 
**Rust** libraries. Its core primitive is the `Pollable` Trait and
achieves asynchrony by using busy polling.

- Have a look at the [overview][3] to see how to build a simple application.
- See the [HTTP Server example][4] for quickly getting up and running.
- View the full [source at GitHub][5]

[1]: https://docs.rs/futures/0.1.18/futures/
[2]: https://docs.rs/tokio/0.1.1/tokio/
[3]: #overview
[4]: #http_server
[5]: https://github.com/gmbeard/server-fx
