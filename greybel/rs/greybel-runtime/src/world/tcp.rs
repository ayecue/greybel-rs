//! Connects programs to a world server over TCP, which shares one world between processes.

use duplex_core::{Channel, ChannelOptions};
use duplex_driver_stdio::StreamDriver;
use greyscript_intrinsics::ServerConnection;
use greyscript_intrinsics::protocol::Program;
use tokio::net::TcpStream;

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
