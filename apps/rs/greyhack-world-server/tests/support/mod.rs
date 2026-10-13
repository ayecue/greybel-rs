//! The game server's process, and programs run against it.

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};

use duplex_core::{Channel, ChannelOptions};
use duplex_driver_stdio::StreamDriver;
use greybel_test_support::interpreter::{inline, snapshot_options};
use greyhack_world::PlayerOptions;
use greyscript_intrinsics::{GreyHackInterpreter, ServerConnection};
use tokio::net::TcpStream;

/// Game-server child process stopped when dropped.
pub struct GameServerProcess {
    child: Child,
    port: u16,
}

impl GameServerProcess {
    pub fn start() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_greyhack-world-server"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("the game server starts");
        let mut line = String::new();

        BufReader::new(child.stdout.as_mut().expect("its output"))
            .read_line(&mut line)
            .expect("it prints its port");

        Self {
            port: line.trim().parse().expect("a port"),
            child,
        }
    }

    pub async fn connect(&self) -> ServerConnection {
        let stream = TcpStream::connect(("127.0.0.1", self.port))
            .await
            .expect("the game server accepts");

        ServerConnection::new(Channel::new(
            &StreamDriver::tcp(stream).expect("the connection is set up"),
            ChannelOptions::default(),
        ))
    }
}

impl Drop for GameServerProcess {
    fn drop(&mut self) {
        drop(self.child.stdin.take());

        let _ = self.child.wait();
    }
}

/// Runs `code` as the default player through `server` and returns its printed output.
pub async fn run(server: &ServerConnection, code: &str) -> String {
    let (lines, options) = snapshot_options();
    let mut program = GreyHackInterpreter::start(
        server,
        &PlayerOptions::default().user,
        options,
        inline(code),
    )
    .await
    .expect("the program starts");

    program.run().await.expect("the program runs to its end");

    lines.text()
}
