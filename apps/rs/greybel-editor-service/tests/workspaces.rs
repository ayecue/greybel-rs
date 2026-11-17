//! Tests saving and loading workspaces.

use axum::http::StatusCode;
use greybel_editor_service::store::MemoryStore;
use serde_json::json;

use crate::support::{TestService, run};

#[test]
fn should_save_a_workspace_and_load_it_by_its_id() {
    run(async {
        let service = TestService::new(MemoryStore::default(), 30);
        let files = json!({ "main.src": "import_code(\"lib/util.src\")", "lib/util.src": "x = 1", "greybel.json": "{}" });
        let saved = service
            .post(
                "/workspaces",
                &json!({ "entry": "main.src", "files": files }),
            )
            .await;
        let id = saved.body["id"].as_str().expect("an id").to_owned();
        let loaded = service.get(&format!("/workspaces/{id}")).await;

        assert_eq!(saved.status, StatusCode::OK, "{}", saved.body);
        assert_eq!(loaded.status, StatusCode::OK);
        assert_eq!(loaded.body["entry"], "main.src");
        assert_eq!(loaded.body["files"], files);
        assert!(loaded.body["createdAt"].is_string());
    });
}

#[test]
fn should_return_the_stored_workspace_for_the_same_files() {
    run(async {
        let store = MemoryStore::default();
        let service = TestService::new(store.clone(), 30);
        let workspace =
            json!({ "entry": "a.src", "files": { "a.src": "print 1", "b.src": "print 2" } });
        let reordered =
            json!({ "entry": "a.src", "files": { "b.src": "print 2", "a.src": "print 1" } });
        let changed =
            json!({ "entry": "b.src", "files": { "a.src": "print 1", "b.src": "print 2" } });
        let first = service.post("/workspaces", &workspace).await;
        let second = service.post("/workspaces", &reordered).await;
        let third = service.post("/workspaces", &changed).await;

        assert_eq!(first.body["id"], second.body["id"]);
        assert_ne!(first.body["id"], third.body["id"]);
        assert_eq!(store.workspace_count(), 2);
    });
}

#[test]
fn should_reject_invalid_workspaces() {
    run(async {
        let service = TestService::new(MemoryStore::default(), 100);
        let cases = [
            (
                json!({ "entry": "main.src", "files": {} }),
                "A workspace needs at least one file.",
            ),
            (
                json!({ "entry": "main.src", "files": { "main.src": "", "../x.src": "" } }),
                "\"../x.src\" is not a valid path",
            ),
            (
                json!({ "entry": "main.src", "files": { "/main.src": "" } }),
                "\"/main.src\" is not a valid path",
            ),
            (
                json!({ "entry": "lib.src", "files": { "main.src": "" } }),
                "The entry \"lib.src\" is not one of the files.",
            ),
            (
                json!({ "entry": "main.src", "files": { "main.src": "x".repeat(1_200_001) } }),
                "more than 1200000 characters",
            ),
        ];

        for (body, message) in cases {
            let response = service.post("/workspaces", &body).await;

            assert_eq!(response.status, StatusCode::BAD_REQUEST, "{body}");
            assert!(
                response.body["message"]
                    .as_str()
                    .unwrap_or_default()
                    .contains(message),
                "{}",
                response.body
            );
        }

        let malformed = service
            .post_text("/workspaces", "{".to_owned(), "1.2.3.4")
            .await;

        assert_eq!(malformed.status, StatusCode::BAD_REQUEST);
        assert!(malformed.body["message"].is_string());
    });
}

#[test]
fn should_answer_404_for_an_unknown_or_malformed_id() {
    run(async {
        let service = TestService::new(MemoryStore::default(), 30);
        let unknown = service
            .get("/workspaces/00000000-0000-4000-8000-000000000000")
            .await;
        let malformed = service.get("/workspaces/not-an-id").await;

        assert_eq!(unknown.status, StatusCode::NOT_FOUND);
        assert_eq!(
            unknown.body["message"],
            "There is no workspace with this id."
        );
        assert_eq!(malformed.status, StatusCode::NOT_FOUND);
    });
}
