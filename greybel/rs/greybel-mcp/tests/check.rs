//! Tests the `check` tool's diagnostics.

use serde_json::json;

use crate::support::{Client, options, run};

#[test]
fn should_check_code_for_problems() {
    run(async {
        let mut client = Client::connect(options()).await;

        let (problems, _) = client
            .call("check", json!({ "code": "x = 1\nif x then" }))
            .await;
        let (none, _) = client.call("check", json!({ "code": "x = 1" })).await;
        let (neither, failed) = client.call("check", json!({})).await;

        assert!(problems.starts_with("code.src:2:"), "{problems}");
        assert!(problems.contains(": error: "), "{problems}");
        assert_eq!(none, "No problems found.");
        assert!(failed, "{neither}");
    });
}
