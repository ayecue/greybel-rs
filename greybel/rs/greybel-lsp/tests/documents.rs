//! Tests open-document edits, diagnostics, and dialect selection.

use greybel_lsp::ReadFileRequest;
use greybel_lsp::types::{self as lsp, Request as _};
use serde_json::json;

use crate::support::{
    Client, MAIN, change_after_the_emoji, client_edited_while_reading, insertion, lsp_position,
    project, project_edited_without_new_diagnostics, project_with_a_missing_reference,
    project_with_an_emoji, project_with_the_reference_defined, uri,
};

#[test]
fn should_report_an_unresolved_reference_on_open() {
    let mut client = project();

    client.open("main.src", "print missing\n");

    assert_eq!(client.codes("main.src"), ["unresolved-reference"]);

    let diagnostic = &client.diagnostics[&uri("main.src")][0];

    assert_eq!(
        diagnostic.range,
        lsp::Range {
            start: lsp_position(0, 6),
            end: lsp_position(0, 13),
        }
    );
    assert_eq!(diagnostic.source.as_deref(), Some("greybel"));
}

#[test]
fn should_clear_diagnostics_after_an_edit_fixing_them() {
    let mut client = project_with_a_missing_reference();

    client.change("main.src", 2, insertion(0, 0, "missing = 1\n"));
    assert!(client.codes("main.src").is_empty());
}

#[test]
fn should_not_send_unchanged_diagnostics_again() {
    let mut client = project_with_the_reference_defined();
    let published = client.published.len();

    client.change("main.src", 3, insertion(1, 13, " "));
    assert_eq!(
        client.published.len(),
        published,
        "unchanged diagnostics are not sent again"
    );
}

#[test]
fn should_clear_diagnostics_on_close() {
    let mut client = project_edited_without_new_diagnostics();

    client.close("main.src");
    assert!(client.codes("main.src").is_empty());
}

#[test]
fn should_take_the_files_content_again_once_its_document_closes() {
    let mut client = project();

    client.open("lib/greet.src", "// No greet any more.\n");
    client.open("main.src", MAIN);
    assert_eq!(client.codes("main.src"), ["unresolved-reference"]);

    client.close("lib/greet.src");
    client.change("main.src", 2, json!([{ "text": MAIN }]));

    assert!(
        client.codes("main.src").is_empty(),
        "the file's own greet is back: {:?}",
        client.codes("main.src")
    );
}

#[test]
fn should_count_utf16_units_in_incremental_changes() {
    let mut client = project_with_an_emoji();

    change_after_the_emoji(&mut client);

    let symbols = client.document_symbols("main.src");

    assert_eq!(symbols[0].range.end, lsp_position(0, 9));
}

#[test]
fn should_report_nothing_after_a_change_counted_in_utf16_units() {
    let mut client = project_with_an_emoji();

    change_after_the_emoji(&mut client);
    assert!(
        client.codes("main.src").is_empty(),
        "{:?}",
        client.diagnostics
    );
}

#[test]
fn should_keep_an_edit_arriving_while_files_are_read() {
    let mut client = client_edited_while_reading();

    client.release_reads();
    assert!(
        client.codes("main.src").is_empty(),
        "{:?}",
        client.codes("main.src")
    );
}

#[test]
fn should_analyze_the_edited_text_not_a_read() {
    let mut client = client_edited_while_reading();

    client.release_reads();

    let symbols = client.document_symbols("main.src");

    assert_eq!(symbols.len(), 1, "the edited text, not a read, is analyzed");
}

#[test]
fn should_check_documents_in_the_dialect_the_server_was_started_with() {
    let mut client = Client::in_dialect(&[], "miniscript");

    client.open("game.ms", "print get_shell\nprint typeof(1)\n");
    assert_eq!(
        client.codes("game.ms"),
        ["unresolved-reference", "unresolved-reference"],
        "neither is MiniScript's"
    );
    assert!(
        client
            .requests
            .iter()
            .all(|method| method != ReadFileRequest::METHOD.as_str())
    );
}

#[test]
fn should_check_a_document_in_the_servers_dialect_whatever_its_language() {
    let mut client = Client::started(&[]);

    client.open_as("game.ms", "print get_shell\n", "miniscript");

    assert_eq!(client.codes("game.ms"), Vec::<String>::new());
}
