use std::time::Duration;

use crate::protocol::{DEFAULT_HOST, DEFAULT_PORT};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientOptions {
    pub host: String,
    pub port: u16,
    pub connect_attempts: u32,
    pub retry_delay: Duration,
    pub request_timeout: Duration,
}

impl Default for ClientOptions {
    fn default() -> Self {
        Self {
            host: DEFAULT_HOST.to_owned(),
            port: DEFAULT_PORT,
            connect_attempts: 3,
            retry_delay: Duration::from_secs(2),
            request_timeout: Duration::from_secs(10),
        }
    }
}

impl ClientOptions {
    #[must_use]
    pub fn with_port(port: u16) -> Self {
        Self {
            port,
            ..Self::default()
        }
    }
}
