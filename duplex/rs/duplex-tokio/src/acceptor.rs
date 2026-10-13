use std::io::{self, ErrorKind};
use std::net::SocketAddr;
use std::time::Duration;

use tokio::net::{TcpListener, TcpStream};

/// Retry delay for resource accept errors such as `EMFILE`, avoiding immediate retries likely to fail again.
const RESOURCE_RETRY_DELAY: Duration = Duration::from_secs(1);

/// Accepts TCP connections, retrying transient failures immediately and resource failures after a delay.
#[derive(Debug)]
pub struct Acceptor {
    listener: TcpListener,
}

impl Acceptor {
    /// Wraps `listener` in a retrying accept loop.
    #[must_use]
    pub fn new(listener: TcpListener) -> Self {
        Self { listener }
    }

    /// Returns the listener local socket address.
    ///
    /// # Errors
    ///
    /// Returns `TcpListener::local_addr` errors.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.listener.local_addr()
    }

    /// Waits for a connection and retries errors using [`Self::retry_delay`].
    pub async fn accept(&self) -> (TcpStream, SocketAddr) {
        loop {
            match self.listener.accept().await {
                Ok(connection) => return connection,
                Err(error) => {
                    let delay = Self::retry_delay(&error);

                    if !delay.is_zero() {
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }
    }

    /// Returns zero for transient connection, interrupted, and would-block errors; other errors use the resource retry
    /// delay.
    fn retry_delay(error: &io::Error) -> Duration {
        match error.kind() {
            ErrorKind::ConnectionAborted
            | ErrorKind::ConnectionReset
            | ErrorKind::ConnectionRefused
            | ErrorKind::Interrupted
            | ErrorKind::WouldBlock => Duration::ZERO,
            _ => RESOURCE_RETRY_DELAY,
        }
    }
}

#[cfg(test)]
#[path = "tests/acceptor.rs"]
mod tests;
