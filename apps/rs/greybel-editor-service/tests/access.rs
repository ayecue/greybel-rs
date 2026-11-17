//! Tests the save limit per client and the allowed origins.

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use greybel_editor_service::api::OriginPolicy;
use greybel_editor_service::store::MemoryStore;
use serde_json::json;

use crate::support::{EDITOR_ORIGIN, TestService, run};

#[test]
fn should_limit_the_saves_of_each_client() {
    run(async {
        let service = TestService::new(MemoryStore::default(), 2);
        let body = |code: &str| json!({ "entry": "main.src", "files": { "main.src": code } });
        let first = service
            .post_from("/workspaces", &body("1"), "1.1.1.1")
            .await;
        let second = service
            .post_from("/code", &json!({ "content": "2" }), "1.1.1.1")
            .await;
        let third = service
            .post_from("/workspaces", &body("3"), "1.1.1.1")
            .await;
        let other = service
            .post_from("/workspaces", &body("4"), "2.2.2.2")
            .await;
        let id = first.body["id"].as_str().expect("an id").to_owned();
        let read = service.get(&format!("/workspaces/{id}")).await;

        assert_eq!(first.status, StatusCode::OK);
        assert_eq!(second.status, StatusCode::OK);
        assert_eq!(third.status, StatusCode::TOO_MANY_REQUESTS);
        assert!(third.retry_after.is_some());
        assert!(
            third.body["message"]
                .as_str()
                .unwrap_or_default()
                .starts_with("Too many saves")
        );
        assert_eq!(other.status, StatusCode::OK);
        assert_eq!(read.status, StatusCode::OK);
    });
}

#[test]
fn should_allow_requests_only_from_the_listed_origins() {
    run(async {
        let service = TestService::new(MemoryStore::default(), 30);
        let request = |origin: &str| {
            Request::get("/workspaces/00000000-0000-4000-8000-000000000000")
                .header(header::ORIGIN, origin)
                .body(Body::empty())
                .expect("a request")
        };
        let listed = service.send(request(EDITOR_ORIGIN)).await;
        let other = service.send(request("https://example.com")).await;

        assert_eq!(listed.allowed_origin.as_deref(), Some(EDITOR_ORIGIN));
        assert_eq!(other.allowed_origin, None);
    });
}

#[test]
fn should_match_origins_with_a_subdomain_pattern() {
    let policy = OriginPolicy::parse("https://editor.greyscript.org, https://*.vscode-cdn.net");

    assert!(policy.allows("https://editor.greyscript.org"));
    assert!(policy.allows("https://1a2b3c.vscode-cdn.net"));
    assert!(policy.allows("https://a.b.vscode-cdn.net"));
    assert!(!policy.allows("https://vscode-cdn.net"));
    assert!(!policy.allows("https://evilvscode-cdn.net"));
    assert!(!policy.allows("http://1a2b3c.vscode-cdn.net"));
    assert!(!policy.allows("https://example.com"));
    assert!(OriginPolicy::parse("*").allows("https://example.com"));
    assert_eq!(
        policy.to_string(),
        "https://editor.greyscript.org,https://*.vscode-cdn.net"
    );
}
