//! Conformance calculator over a WebSocket for TypeScript package tests. `ALLOW_ORIGIN` configures the accepted
//! `Origin` header.

use duplex_driver_websocket::{WebSocketOptions, WebSocketServer};
use duplex_test_support::conformance::serve_calculator;
use duplex_test_support::local::run_until_input_ends;

async fn serve() {
    let options = WebSocketOptions {
        allow_origin: std::env::var("ALLOW_ORIGIN").ok(),
        ..WebSocketOptions::default()
    };
    let mut server = WebSocketServer::bind("127.0.0.1:0", options)
        .await
        .expect("the WebSocket server listens");

    println!("listening ws://{}/duplex", server.local_addr());
    while let Some(transport) = server.accept().await {
        serve_calculator(transport);
    }
}

fn main() {
    run_until_input_ends(serve());
}
