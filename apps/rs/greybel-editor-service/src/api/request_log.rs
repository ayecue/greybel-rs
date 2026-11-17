use std::time::Duration;

use axum::http::{Request, Response};
use tower_http::trace::{MakeSpan, OnResponse};
use tracing::Span;

use super::ClientAddress;

/// A span per request with its method, path and client IP; every log of the request carries them.
#[derive(Debug, Clone)]
pub struct RequestSpan {
    client: ClientAddress,
}

impl RequestSpan {
    pub fn new(client: ClientAddress) -> Self {
        Self { client }
    }
}

impl<B> MakeSpan<B> for RequestSpan {
    fn make_span(&mut self, request: &Request<B>) -> Span {
        tracing::info_span!(
            "request",
            method = %request.method(),
            path = %request.uri().path(),
            client = %self.client.find(request),
        )
    }
}

/// Logs each response's status and latency in milliseconds; server errors at error level.
#[derive(Debug, Clone, Copy, Default)]
pub struct ResponseLog;

impl<B> OnResponse<B> for ResponseLog {
    fn on_response(self, response: &Response<B>, latency: Duration, _span: &Span) {
        let status = response.status().as_u16();
        let latency_ms = (latency.as_secs_f64() * 100_000.0).round() / 100.0;

        if response.status().is_server_error() {
            tracing::error!(status, latency_ms, "response");
        } else {
            tracing::info!(status, latency_ms, "response");
        }
    }
}
