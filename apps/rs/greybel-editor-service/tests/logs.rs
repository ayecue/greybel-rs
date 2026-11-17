//! Tests what requests log.

use greybel_editor_service::store::MemoryStore;
use serde_json::json;

use crate::support::TestService;
use crate::support::logs::{LogCapture, SECRET_CODE};

#[test]
fn should_log_saves_rejections_and_responses_without_file_contents() {
    let logs = LogCapture::default();

    logs.run(async {
        let service = TestService::new(MemoryStore::default(), 3);
        let workspace = json!({ "entry": "main.src", "files": { "main.src": SECRET_CODE, "lib.src": "x = 1" } });

        service.post_from("/workspaces", &workspace, "1.1.1.1").await;
        service.post_from("/workspaces", &workspace, "1.1.1.1").await;
        service
            .post_from("/workspaces", &json!({ "entry": "main.src", "files": {} }), "1.1.1.1")
            .await;
        service.post_from("/workspaces", &workspace, "1.1.1.1").await;
        service.get("/workspaces/not-an-id").await;
        service.get("/health").await;
    });

    let text = logs.text();
    let saves = logs.find("workspace saved");
    let rejections = logs.find("workspace rejected");
    let limits = logs.find("save limit reached");
    let responses = logs.find(": response ");

    assert_eq!(saves.len(), 2, "{text}");
    assert!(
        saves[0].contains("files=2 characters=31 is_new=true"),
        "{}",
        saves[0]
    );
    assert!(saves[1].contains("is_new=false"), "{}", saves[1]);
    assert!(
        saves[0].contains("request{method=POST path=/workspaces client=1.1.1.1}"),
        "{}",
        saves[0]
    );
    assert_eq!(rejections.len(), 1, "{text}");
    assert!(
        rejections[0].contains("reason=A workspace needs at least one file."),
        "{}",
        rejections[0]
    );
    assert_eq!(limits.len(), 1, "{text}");
    assert!(limits[0].contains(" WARN "), "{}", limits[0]);
    assert!(limits[0].contains("retry_after_s="), "{}", limits[0]);
    assert_eq!(responses.len(), 5, "{text}");
    assert!(
        responses.iter().any(|line| line.contains("status=429")),
        "{text}"
    );
    assert!(
        responses
            .iter()
            .any(|line| line.contains("path=/workspaces/not-an-id") && line.contains("status=404")),
        "{text}"
    );
    assert!(
        responses.iter().all(|line| line.contains("latency_ms=")),
        "{text}"
    );
    assert!(logs.find("/health").is_empty(), "{text}");
    assert!(logs.find("secret token").is_empty(), "{text}");
}
