use std::net::SocketAddr;

use axum::extract::ConnectInfo;
use axum::http::{HeaderMap, Request};

/// Finds a request's client IP: the first address in `header`, which a proxy such as App Platform
/// (`do-connecting-ip`) sets, else the connection's address.
#[derive(Debug, Clone, Default)]
pub struct ClientAddress {
    header: Option<String>,
}

impl ClientAddress {
    const UNKNOWN: &str = "unknown";

    pub fn new(header: Option<String>) -> Self {
        Self { header }
    }

    pub fn find<B>(&self, request: &Request<B>) -> String {
        self.header
            .as_deref()
            .and_then(|name| Self::get_header(request.headers(), name))
            .or_else(|| {
                request
                    .extensions()
                    .get::<ConnectInfo<SocketAddr>>()
                    .map(|info| info.0.ip().to_string())
            })
            .unwrap_or_else(|| Self::UNKNOWN.to_owned())
    }

    fn get_header(headers: &HeaderMap, name: &str) -> Option<String> {
        let value = headers.get(name)?.to_str().ok()?;
        let client = value.split(',').next()?.trim();

        (!client.is_empty()).then(|| client.to_owned())
    }
}
