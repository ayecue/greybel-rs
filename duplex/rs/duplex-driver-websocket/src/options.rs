const MEBIBYTE: usize = 1024 * 1024;

/// `WebSocketServer` options.
#[derive(Debug, Clone)]
pub struct WebSocketOptions {
    /// Client connection URL path.
    pub path: String,
    /// Allowed `Origin` value. `*` accepts any origin; `None` requires origin authority to match `Host`. Requests without
    /// `Origin` are always accepted.
    pub allow_origin: Option<String>,
    /// Maximum incoming batch bytes. Larger messages close the transport.
    pub max_batch: usize,
    /// Maximum queued unwritten outbound bytes. Exceeding the limit disconnects the client.
    pub max_pending_bytes: usize,
}

impl Default for WebSocketOptions {
    fn default() -> Self {
        Self {
            path: "/duplex".to_owned(),
            allow_origin: None,
            max_batch: 16 * MEBIBYTE,
            max_pending_bytes: 16 * MEBIBYTE,
        }
    }
}

impl WebSocketOptions {
    pub(crate) fn allows(&self, origin: Option<&str>, host: Option<&str>) -> bool {
        let Some(origin) = origin else {
            return true;
        };

        match self.allow_origin.as_deref() {
            Some("*") => true,
            Some(allowed_origin) => origin == allowed_origin,
            None => host.is_some_and(|host| {
                origin
                    .split_once("://")
                    .is_some_and(|(_, authority)| authority == host)
            }),
        }
    }
}

#[cfg(test)]
#[path = "tests/options.rs"]
mod tests;
