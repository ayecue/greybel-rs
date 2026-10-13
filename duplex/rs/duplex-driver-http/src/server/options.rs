use std::time::Duration;

const MEBIBYTE: usize = 1024 * 1024;

/// `HttpServer` options.
#[derive(Debug, Clone)]
pub struct HttpOptions {
    /// Base POST and long-poll path. Event streams use `<path>/events`; byte streams use `<path>/stream`.
    pub path: String,
    /// Maximum long-poll batch wait before returning no content.
    pub poll_timeout: Duration,
    /// Request idle timeout for transports without an open stream.
    pub idle: Duration,
    /// Heartbeat record interval for idle open streams.
    pub heartbeat: Duration,
    /// `Access-Control-Allow-Origin` value. `*` permits any origin; `None` omits CORS headers.
    pub allow_origin: Option<String>,
    /// `Access-Control-Max-Age` value. `None` uses the browser default.
    pub preflight_max_age: Option<Duration>,
    /// Maximum POST body bytes. Larger bodies receive HTTP 413 without processing.
    pub max_body: usize,
    /// Maximum retained bytes for unacknowledged downstream batches. Exceeding the limit disconnects the transport.
    pub max_backlog: usize,
    /// Maximum concurrent client transports. Excess open requests receive HTTP 503.
    pub max_transports: usize,
}

impl Default for HttpOptions {
    fn default() -> Self {
        Self {
            path: "/duplex".to_owned(),
            poll_timeout: Duration::from_secs(25),
            idle: Duration::from_secs(60),
            heartbeat: Duration::from_secs(15),
            allow_origin: None,
            preflight_max_age: Some(Duration::from_secs(600)),
            max_body: 16 * MEBIBYTE,
            max_backlog: 16 * MEBIBYTE,
            max_transports: 4096,
        }
    }
}
