Async Server Framework
===
*Server-Fx* is a framework for building asynchronous network 
servers in Rust. It doesn't use event-based IO. Instead it uses
busy polling on background threads. Although it doesn't use 
[Futures][1] or [Tokio][2], it borrows a lot of their concepts.

*Server-Fx is a WIP and isn't production ready in it's current 
state - The HTTP parser is a bit hand-wavey, for example.*

Current Performance
---
These figures are based on the `simple_http` example, running
on a rather dated Macbook Pro 5,4 (Core 2 Duo, 8GB)

    ./wrk -t5 -c500 -d1m http://localhost:5050/content/overview
    Running 1m test @ http://localhost:5050/content/overview
      5 threads and 500 connections
      Thread Stats   Avg      Stdev     Max   +/- Stdev
        Latency    29.56ms   10.69ms 367.95ms   83.71%
        Req/Sec     3.43k   387.94     6.26k    81.37%
      1020232 requests in 1.00m, 1.10GB read
    Requests/sec:  16977.14
    Transfer/sec:     18.75MB

[1]: https://docs.rs/futures/0.1.18/futures
[2]: https://tokio.rs/
