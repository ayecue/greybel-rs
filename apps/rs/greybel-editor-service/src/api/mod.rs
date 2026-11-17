mod client;
mod error;
mod legacy;
mod rate_limit;
mod request_log;
mod workspaces;

use axum::Router;
use axum::extract::{DefaultBodyLimit, Request, State};
use axum::http::{HeaderValue, Method, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
pub use client::ClientAddress;
pub use error::ApiError;
pub use rate_limit::RateLimiter;
pub use request_log::{RequestSpan, ResponseLog};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::store::WorkspaceStore;

/// The HTTP API over a workspace store.
#[derive(Clone)]
pub struct Api<S> {
    store: S,
    limiter: RateLimiter,
}

/// Which origins browsers may call the API from: every origin, or the listed ones. A listed origin may start its
/// host with `*.` to allow every subdomain, such as `https://*.vscode-cdn.net` for VS Code for the Web.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OriginPolicy {
    Any,
    List(Vec<String>),
}

impl<S: WorkspaceStore> Api<S> {
    /// Request bodies above this size are rejected; the character limit of `Workspace` applies after decoding.
    pub const BODY_LIMIT: usize = 8 * 1024 * 1024;

    pub fn new(store: S, limiter: RateLimiter) -> Self {
        Self { store, limiter }
    }

    /// `/health` is outside the request log, so App Platform's health checks don't fill it.
    pub fn router(self, origins: &OriginPolicy) -> Router {
        let saves = Router::new()
            .route("/workspaces", axum::routing::post(Self::save_workspace))
            .route("/code", axum::routing::post(Self::save_code))
            .route_layer(middleware::from_fn_with_state(
                self.clone(),
                Self::limit_saves,
            ));

        let request_log = TraceLayer::new_for_http()
            .make_span_with(RequestSpan::new(self.limiter.client_address().clone()))
            .on_request(())
            .on_response(ResponseLog)
            .on_failure(());

        Router::new()
            .route("/workspaces/{id}", get(Self::find_workspace))
            .route("/code/{id}", get(Self::find_code))
            .merge(saves)
            .layer(DefaultBodyLimit::max(Self::BODY_LIMIT))
            .layer(Self::create_cors(origins))
            .layer(request_log)
            .route("/health", get(Self::check_health))
            .with_state(self)
    }

    async fn limit_saves(State(api): State<Self>, request: Request, next: Next) -> Response {
        match api.limiter.check(&request) {
            Ok(()) => next.run(request).await,
            Err(wait) => ApiError::SaveLimit(wait).into_response(),
        }
    }

    async fn check_health(State(api): State<Self>) -> Result<StatusCode, ApiError> {
        api.store.check().await?;

        Ok(StatusCode::OK)
    }

    fn create_cors(origins: &OriginPolicy) -> CorsLayer {
        let allow_origin = match origins {
            OriginPolicy::Any => AllowOrigin::any(),
            OriginPolicy::List(_) => {
                let policy = origins.clone();

                AllowOrigin::predicate(move |origin: &HeaderValue, _| {
                    origin.to_str().is_ok_and(|origin| policy.allows(origin))
                })
            }
        };

        CorsLayer::new()
            .allow_origin(allow_origin)
            .allow_methods([Method::GET, Method::POST])
            .allow_headers([header::CONTENT_TYPE])
    }
}

/// As written in `ALLOWED_ORIGINS`.
impl std::fmt::Display for OriginPolicy {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Any => formatter.write_str("*"),
            Self::List(origins) => formatter.write_str(&origins.join(",")),
        }
    }
}

impl OriginPolicy {
    pub fn allows(&self, origin: &str) -> bool {
        match self {
            Self::Any => true,
            Self::List(origins) => origins.iter().any(|pattern| Self::matches(pattern, origin)),
        }
    }

    fn matches(pattern: &str, origin: &str) -> bool {
        let Some((scheme, host)) = pattern.split_once("://*.") else {
            return pattern == origin;
        };

        origin
            .strip_prefix(scheme)
            .and_then(|tail| tail.strip_prefix("://"))
            .and_then(|origin_host| origin_host.strip_suffix(host))
            .is_some_and(|subdomain| subdomain.len() > 1 && subdomain.ends_with('.'))
    }

    /// `*` for every origin, else a comma-separated list such as `https://editor.greyscript.org`.
    pub fn parse(text: &str) -> Self {
        if text.trim() == "*" {
            return Self::Any;
        }

        Self::List(
            text.split(',')
                .map(|origin| origin.trim().trim_end_matches('/').to_owned())
                .filter(|origin| !origin.is_empty())
                .collect(),
        )
    }
}
