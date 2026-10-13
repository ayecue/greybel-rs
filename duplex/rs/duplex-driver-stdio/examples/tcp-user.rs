//! Conformance calculator client for C# core tests. Connects to the supplied TCP port, sends a log message, prints the
//! reported progress phase, then prints the calculator's close reason.

use duplex_core::{Channel, ChannelOptions};
use duplex_driver_stdio::StreamDriver;
use duplex_test_support::conformance::user::{Log, Progress, User};
use duplex_test_support::local::run_local;
use tokio::net::TcpStream;
use tokio::sync::mpsc;

fn main() {
    let port: u16 = std::env::args()
        .nth(1)
        .and_then(|port| port.parse().ok())
        .expect("the calculator's port is given");

    run_local(async move {
        let stream = TcpStream::connect(("127.0.0.1", port))
            .await
            .expect("the calculator accepts");
        let transport = StreamDriver::tcp(stream).expect("the connection is set up");
        let channel = Channel::<User, _>::new(&transport, ChannelOptions::default());
        let (progress, mut phases) = mpsc::unbounded_channel();

        channel.on::<Progress>(move |message, _next| {
            let _ = progress.send(format!("{:?}", message.phase));
        });
        channel
            .send(&Log {
                text: "from rust".into(),
            })
            .expect("the log is sent");

        let phase = phases
            .recv()
            .await
            .expect("the calculator sends its progress");

        println!("{phase}");
        println!("{:?}", channel.close_event().await);
    });
}
