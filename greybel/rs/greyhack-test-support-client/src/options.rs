use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestClientOptions {
    pub host: String,
    pub port: u16,
    pub request_timeout: Duration,
}

impl Default for TestClientOptions {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_owned(),
            port: 0,
            request_timeout: Duration::from_secs(10),
        }
    }
}

impl TestClientOptions {
    #[must_use]
    pub fn with_port(port: u16) -> Self {
        Self {
            port,
            ..Self::default()
        }
    }
}
