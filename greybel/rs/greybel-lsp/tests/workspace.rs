//! Tests client-hosted workspace files, includes, imports, and later file creation.

use greybel_lsp::types::Request as _;
use greybel_lsp::{FileSearch, ReadFile};
use serde_json::json;

use crate::support::{
    Client, LIBRARY, MAIN, client_deferring_reads, client_missing_the_library, project, uri,
};

#[test]
fn should_read_includes_through_the_client() {
    let mut client = project();

    client.requests.clear();
    client.open("main.src", MAIN);
    assert!(
        client.codes("main.src").is_empty(),
        "{:?}",
        client.codes("main.src")
    );
}

#[test]
fn should_share_one_read_between_the_scan_and_an_include() {
    let mut client = client_deferring_reads();

    client.open("main.src", MAIN);
    assert_eq!(
        client.pending_requests.len(),
        1,
        "the workspace scan and the include share one read"
    );
    assert!(
        !client.published.contains(&uri("main.src")),
        "no diagnostics while the include is loading"
    );
}

#[test]
fn should_report_a_missing_include_on_open() {
    let mut client = Client::started(&[]);

    client.open("main.src", MAIN);
    assert_eq!(
        client.codes("main.src"),
        ["missing-import", "unresolved-reference"]
    );
}

#[test]
fn should_read_a_missing_include_only_once() {
    let mut client = client_missing_the_library();

    client.change(
        "main.src",
        2,
        json!([{ "text": format!("{MAIN}print 1\n") }]),
    );
    client.change("main.src", 3, json!([{ "text": MAIN }]));

    let reads = client
        .requests
        .iter()
        .filter(|method| *method == ReadFile::METHOD.as_str())
        .count();

    assert_eq!(
        reads, 1,
        "the workspace remembers that the include is missing"
    );
}

#[test]
fn should_resolve_an_include_of_a_file_created_later() {
    let mut client = client_missing_the_library();

    client
        .files
        .insert(uri("lib/greet.src"), LIBRARY.to_owned());
    client.notify_value(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [{ "uri": uri("lib/greet.src"), "type": 1 }] }),
    );
    assert!(
        client.codes("main.src").is_empty(),
        "{:?}",
        client.codes("main.src")
    );
}

#[test]
fn should_report_an_unresolved_reference_to_another_file() {
    let mut client = project();

    client.open("other.src", "print greet(\"x\")\n");
    assert_eq!(client.codes("other.src"), ["unresolved-reference"]);
}

#[test]
fn should_check_a_library_of_a_root_file_inside_it() {
    let mut client = Client::with_settings(
        &[
            ("main.src", "#include \"a.src\"\n#include \"b.src\"\n"),
            ("a.src", "helper = function(x)\n  return x\nend function\n"),
            ("b.src", "print helper(1)\n"),
        ],
        json!({}),
    );

    client.open("b.src", "print helper(1)\n");

    assert!(client.codes("b.src").is_empty(), "{:?}", client.diagnostics);
}

#[test]
fn should_take_a_created_file_only_once_the_client_lists_it() {
    let mut client = client_missing_the_library();

    client.requests.clear();
    client.notify_value(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [{ "uri": uri("node_modules/lib.src"), "type": 1 }] }),
    );

    assert_eq!(
        client.requests,
        [FileSearch::METHOD.as_str()],
        "the client lists the workspace's files again, without the excluded one"
    );
}

#[test]
fn should_ignore_a_change_to_a_file_outside_the_workspace_files() {
    let mut client = project();

    client.requests.clear();
    client.notify_value(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [{ "uri": uri("notes.txt"), "type": 2 }] }),
    );

    assert!(client.requests.is_empty(), "{:?}", client.requests);
}
