use std::time::Duration;

use duplex_core::ErrorCode;

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("could not connect to {addr} after {attempts} attempt(s): {source}")]
    Connect {
        addr: String,
        attempts: u32,
        source: std::io::Error,
    },
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("connection closed")]
    Disconnect,
    #[error("the plugin refused the channel: {0:?}")]
    Refusal(ErrorCode),
    #[error(transparent)]
    Channel(#[from] duplex_core::Error),
    #[error("didn't receive response within {}ms", .0.as_millis())]
    Timeout(Duration),
    #[error("unexpected response {0}")]
    UnexpectedResponse(&'static str),
}
