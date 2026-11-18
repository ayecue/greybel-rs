//! Tests `api_search` and `api_reference`.

use serde_json::json;

use crate::support::{Client, options, run};

#[test]
fn should_read_the_api_reference() {
    run(async {
        let mut client = Client::connect(options()).await;

        let (matches, _) = client
            .call("api_search", json!({ "query": "shell.host" }))
            .await;
        let (member, _) = client
            .call("api_reference", json!({ "name": "shell.host_computer" }))
            .await;
        let (section, _) = client
            .call("api_reference", json!({ "name": "shell" }))
            .await;
        let (missing, failed) = client
            .call("api_reference", json!({ "name": "nothing" }))
            .await;

        assert!(
            matches.lines().any(|line| line == "shell.host_computer"),
            "{matches}"
        );
        assert!(
            member.starts_with("## `shell.host_computer(): computer`"),
            "{member}"
        );
        assert!(section.starts_with("# shell\n"), "{section}");
        assert!(failed, "{missing}");
    });
}
