//! Tests standalone code transformation.

use serde_json::json;

use crate::support::{Client, options, run};

#[test]
fn should_transform_code() {
    run(async {
        let mut client = Client::connect(options()).await;

        let (uglified, failed) = client
            .call(
                "transform",
                json!({ "code": "value = 1 // a comment\nprint(value)", "mode": "uglify" }),
            )
            .await;

        assert!(!failed, "{uglified}");
        assert!(!uglified.contains("comment"), "{uglified}");
        assert!(!uglified.contains("value"), "{uglified}");
    });
}
