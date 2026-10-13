use duplex_core::{Channel, ChannelOptions, ErrorCode};
use duplex_driver_websocket::{WebSocketOptions, WebSocketServer};
use duplex_test_support::conformance::calculator;
use duplex_test_support::local::run_local;
use tokio_tungstenite::tungstenite::http::StatusCode;

use crate::support::server::{connect, refusal_status};

#[test]
fn should_refuse_pages_of_origins_it_does_not_allow_and_other_paths() {
    run_local(async {
        let options = WebSocketOptions {
            allow_origin: Some("http://pages.test".to_owned()),
            ..WebSocketOptions::default()
        };
        let server = WebSocketServer::bind("127.0.0.1:0", options)
            .await
            .expect("the websocket server binds to a local port");
        let elsewhere = refusal_status(connect(server.local_addr(), "/other", None).await);
        let other_origin =
            refusal_status(connect(server.local_addr(), "/duplex", Some("http://evil.test")).await);
        let allowed = connect(server.local_addr(), "/duplex", Some("http://pages.test")).await;

        assert_eq!(elsewhere, Some(StatusCode::NOT_FOUND));
        assert_eq!(other_origin, Some(StatusCode::FORBIDDEN));
        assert!(allowed.is_ok());
    });
}

#[test]
fn should_give_the_hub_of_each_client_until_it_goes() {
    run_local(async {
        let mut server = WebSocketServer::bind("127.0.0.1:0", WebSocketOptions::default())
            .await
            .expect("the websocket server binds to a local port");
        let client = connect(server.local_addr(), "/duplex", None)
            .await
            .expect("the websocket handshake succeeds");
        let transport = server
            .accept()
            .await
            .expect("the server accepts the client transport");
        let channel = Channel::<calculator::Calculator, _>::new(
            &transport,
            ChannelOptions::without_health_checks(),
        );

        drop(client);

        let reason = channel.close_event().await;

        assert_eq!(reason, ErrorCode::ConnectionLoss);
    });
}
