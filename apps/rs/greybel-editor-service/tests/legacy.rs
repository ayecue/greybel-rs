//! Tests the legacy `/code` API and projects greybel-editor-be saved.

use axum::http::StatusCode;
use greybel_editor_service::store::MemoryStore;
use serde_json::json;
use uuid::Uuid;

use crate::support::{TestService, run};

#[test]
fn should_read_projects_of_the_legacy_table_through_both_apis() {
    run(async {
        let store = MemoryStore::default();
        let code = Uuid::new_v4();
        let project = Uuid::new_v4();
        let document = r#"{"greybel":"project","version":1,"entry":"a.src","files":{"a.src":"print 1","b.src":"print 2"}}"#;

        store.insert_legacy(code, "print(\"hi\")");
        store.insert_legacy(project, document);

        let service = TestService::new(store, 30);
        let code_workspace = service.get(&format!("/workspaces/{code}")).await;
        let project_workspace = service.get(&format!("/workspaces/{project}")).await;
        let code_content = service.get(&format!("/code/{code}")).await;

        assert_eq!(code_workspace.body["entry"], "main.src");
        assert_eq!(
            code_workspace.body["files"],
            json!({ "main.src": "print(\"hi\")" })
        );
        assert_eq!(project_workspace.body["entry"], "a.src");
        assert_eq!(
            project_workspace.body["files"],
            json!({ "a.src": "print 1", "b.src": "print 2" })
        );
        assert_eq!(code_content.status, StatusCode::OK);
        assert_eq!(code_content.body["content"], "print(\"hi\")");
    });
}

#[test]
fn should_save_legacy_content_as_a_workspace() {
    run(async {
        let service = TestService::new(MemoryStore::default(), 30);
        let document = r#"{"greybel":"project","version":1,"entry":"a.src","files":{"a.src":"print 1","b.src":"print 2"}}"#;
        let code = service
            .post("/code", &json!({ "content": "print 1" }))
            .await;
        let project = service.post("/code", &json!({ "content": document })).await;
        let code_id = code.body["id"].as_str().expect("an id").to_owned();
        let project_id = project.body["id"].as_str().expect("an id").to_owned();
        let code_workspace = service.get(&format!("/workspaces/{code_id}")).await;
        let project_workspace = service.get(&format!("/workspaces/{project_id}")).await;
        let project_content = service.get(&format!("/code/{project_id}")).await;
        let missing = service.post("/code", &json!({})).await;

        assert_eq!(code.status, StatusCode::OK);
        assert_eq!(code.body["content"], "print 1");
        assert_eq!(
            code_workspace.body["files"],
            json!({ "main.src": "print 1" })
        );
        assert_eq!(project_workspace.body["entry"], "a.src");
        assert_eq!(project_content.body["content"], document);
        assert_eq!(missing.status, StatusCode::UNPROCESSABLE_ENTITY);
    });
}
