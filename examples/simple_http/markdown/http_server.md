A Simple HTTP Server
===

The main extension point of *Server Fx* is the `Handler` trait. Here is a
simple implementation that always returns a `200 OK` response, with the
contents of `<h1>Hello, World</h1>`. *You will note that although the
example sets the `Content-Type` header, it doesn't set the `Content-Length`
header. This is done automatically by the framework*.

```rust
use std::io;
use server_fx::http::types::Request as HttpRequest;
use server_fx::http::types::Response as HttpRequest;
use server_fx::http::types::BodyChunk;
use server_fx::http::types::ResponseBuilder;
use server_fx::handler::Handler;

struct SimpleHandler;

impl Handler for SimpleHandler {
    type Request = HttpRequest;
    type Response = (HttpResponse, BodyChunk);
    type Error = io::Error;
    type Pollable = Result<Self::Response, Self::Error>;

    fn handle(&self, _: HttpRequest) -> Self::Pollable {
        let mut response = ResponseBuilder::new(200, "OK")
            .build_with_buffer("<h1>Hello, World</h1>");
        response.add_header("Content-Type", "text/html");
        Ok(response)
    }
}
```

Once a *handler* has been implemented, a server instance can be created

> TODO: Implement `HttpProto`

```rust
fn main() {
    TcpServer::new(HttpProto)
        .serve("127.0.0.1:5050", || SimpleHandler)
        .unwrap();
}
```

This starts a local server listening on port `5050`. the `serve()` function
blocks indefinitely.
