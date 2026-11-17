//! An in-process service and its requests.

pub mod logs;

use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use greybel_editor_service::api::{Api, OriginPolicy, RateLimiter};
use greybel_editor_service::store::WorkspaceStore;
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

pub const EDITOR_ORIGIN: &str = "https://editor.greyscript.org";

/// The API over `store`, allowing `saves_per_minute` saves per `x-client-ip`.
pub struct TestService {
    router: Router,
}

/// A response's status, `Retry-After` and JSON body.
pub struct TestResponse {
    pub status: StatusCode,
    pub retry_after: Option<String>,
    pub allowed_origin: Option<String>,
    pub body: Value,
}

impl TestService {
    pub fn new(store: impl WorkspaceStore, saves_per_minute: u32) -> Self {
        let limiter = RateLimiter::new(
            saves_per_minute,
            Duration::from_secs(60),
            Some("x-client-ip".to_owned()),
        );

        Self {
            router: Api::new(store, limiter)
                .router(&OriginPolicy::List(vec![EDITOR_ORIGIN.to_owned()])),
        }
    }

    pub async fn get(&self, path: &str) -> TestResponse {
        self.send(Request::get(path).body(Body::empty()).expect("a request"))
            .await
    }

    pub async fn post(&self, path: &str, body: &Value) -> TestResponse {
        self.post_from(path, body, "1.2.3.4").await
    }

    pub async fn post_from(&self, path: &str, body: &Value, client: &str) -> TestResponse {
        self.post_text(path, body.to_string(), client).await
    }

    pub async fn post_text(&self, path: &str, body: String, client: &str) -> TestResponse {
        self.send(
            Request::post(path)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::ORIGIN, EDITOR_ORIGIN)
                .header("x-client-ip", client)
                .body(Body::from(body))
                .expect("a request"),
        )
        .await
    }

    pub async fn send(&self, request: Request<Body>) -> TestResponse {
        let response = self
            .router
            .clone()
            .oneshot(request)
            .await
            .expect("a response");
        let status = response.status();
        let header_text = |name| {
            response
                .headers()
                .get(name)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned)
        };
        let retry_after = header_text(header::RETRY_AFTER);
        let allowed_origin = header_text(header::ACCESS_CONTROL_ALLOW_ORIGIN);
        let bytes = response
            .into_body()
            .collect()
            .await
            .expect("a body")
            .to_bytes();
        let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);

        TestResponse {
            status,
            retry_after,
            allowed_origin,
            body,
        }
    }
}

/// Runs `future` on a current-thread runtime.
pub fn run<F: Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a runtime")
        .block_on(future)
}
