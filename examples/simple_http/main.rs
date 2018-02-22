extern crate server_fx;

mod handler;
mod proto;
mod content_handler;

use server_fx::http::types;
use server_fx::server::TcpServer;
use server_fx::http::router::{
    Route, 
    Router, 
};

use handler::{HttpServer, SimpleHtmlRouteHandler};
use proto::HttpProto;
use content_handler::ContentRouteHandler;

fn main() {
    let routes = vec![
        Route::new(
            types::HttpMethod::Get, 
            "/static/*", 
            SimpleHtmlRouteHandler::new("./examples/simple_http"),
        ),
        Route::new(
            types::HttpMethod::Get,
            "/content/:page",
            ContentRouteHandler::new("./examples/simple_http/markdown"),
        ),
    ];

    TcpServer::new(HttpProto)
        .serve("127.0.0.1:5050", move || HttpServer(Router::new(routes)))
        .unwrap();
}
