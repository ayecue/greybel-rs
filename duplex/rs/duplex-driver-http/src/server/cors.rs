use std::time::Duration;

use hyper::Response;
use hyper::header::{self, HeaderValue};

use crate::client::TRANSPORT_HEADER;
use crate::downstream::ResponseBody;

const ALLOWED_METHODS: &str = "GET, POST, OPTIONS";
const ALLOWED_HEADERS: &str = "duplex-transport, duplex-received, content-type";
const EXPOSED_HEADERS: &str = "duplex-transport";

/// CORS response headers precomputed from server options.
pub(crate) struct Cors {
    origin: Option<HeaderValue>,
    max_age: Option<HeaderValue>,
}

impl Cors {
    pub(crate) fn new(allow_origin: Option<&str>, max_age: Option<Duration>) -> Self {
        Self {
            origin: allow_origin.and_then(|origin| HeaderValue::from_str(origin).ok()),
            max_age: max_age.map(|max_age| HeaderValue::from(max_age.as_secs())),
        }
    }

    /// Adds the configured origin and preflight method, header, and max-age directives.
    /// Exposes the transport header when present in a response.
    pub(crate) fn apply(&self, response: &mut Response<ResponseBody>, preflight: bool) {
        let Some(origin) = &self.origin else {
            return;
        };
        let headers = response.headers_mut();

        headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin.clone());
        if !preflight {
            if headers.contains_key(TRANSPORT_HEADER) {
                headers.insert(
                    header::ACCESS_CONTROL_EXPOSE_HEADERS,
                    HeaderValue::from_static(EXPOSED_HEADERS),
                );
            }

            return;
        }

        headers.insert(
            header::ACCESS_CONTROL_ALLOW_HEADERS,
            HeaderValue::from_static(ALLOWED_HEADERS),
        );
        headers.insert(
            header::ACCESS_CONTROL_ALLOW_METHODS,
            HeaderValue::from_static(ALLOWED_METHODS),
        );
        if let Some(max_age) = &self.max_age {
            headers.insert(header::ACCESS_CONTROL_MAX_AGE, max_age.clone());
        }
    }
}

#[cfg(test)]
#[path = "tests/cors.rs"]
mod tests;
