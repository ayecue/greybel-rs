//! Conformance calculator for TypeScript driver tests. Select `http` long polling, `sse` event streams, or `stream`
//! binary streaming; `ALLOW_ORIGIN` configures CORS.

use duplex_driver_http::{ByteStream, Downstream, EventStream, HttpOptions, HttpServer, LongPoll};
use duplex_test_support::conformance::serve_calculator;
use duplex_test_support::local::run_until_input_ends;

async fn serve<M: Downstream>() {
    let options = HttpOptions {
        allow_origin: std::env::var("ALLOW_ORIGIN").ok(),
        ..HttpOptions::default()
    };
    let mut server = HttpServer::<M>::bind("127.0.0.1:0", options)
        .await
        .expect("the HTTP server listens");

    println!("listening http://{}/duplex", server.local_addr());
    while let Some(transport) = server.accept().await {
        serve_calculator(transport);
    }
}

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("sse") => run_until_input_ends(serve::<EventStream>()),
        Some("stream") => run_until_input_ends(serve::<ByteStream>()),
        _ => run_until_input_ends(serve::<LongPoll>()),
    }
}
