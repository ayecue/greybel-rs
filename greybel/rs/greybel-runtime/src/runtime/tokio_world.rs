//! Tokio helpers for connecting programs to in-process or TCP world servers. TCP allows cross-process world sharing.

use duplex_core::{Channel, ChannelOptions};
use duplex_driver_stdio::StreamDriver;
use greyhack_world_operations::WorldServer;
use greyscript_intrinsics::ServerConnection;
use greyscript_intrinsics::protocol::Program;
use tokio::net::TcpStream;

/// Creates an in-memory `server` connection on the current Tokio `LocalSet` for hosts without a transport.
#[must_use]
pub fn connect_on_tokio(server: &WorldServer) -> ServerConnection {
    ServerConnection::new(Channel::<Program, _>::new(
        &server.connect_on_tokio(),
        ChannelOptions::default(),
    ))
}

/// Connects to the world server at `address` on the current Tokio `LocalSet`.
///
/// # Errors
///
/// Returns an I/O error when TCP connection or stream setup fails.
pub async fn connect_over_tcp(address: &str) -> std::io::Result<ServerConnection> {
    let transport = StreamDriver::tcp(TcpStream::connect(address).await?)?;

    Ok(ServerConnection::new(
        Channel::<Program, StreamDriver>::new(&transport, ChannelOptions::default()),
    ))
}
