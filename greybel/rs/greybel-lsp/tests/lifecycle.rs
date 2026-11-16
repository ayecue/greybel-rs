//! Tests initialization, shutdown, exit, and requests rejected before or after each lifecycle state.

use greybel_lsp::FileSearch;
use greybel_lsp::types::{self as lsp, ErrorCodes, Request as _};
use serde_json::json;

use crate::support::{
    Client, client_asked_an_unknown_method, client_refused_before_initialize, client_shut_down,
};

#[test]
fn should_refuse_requests_before_initialize() {
    let mut client = Client::new(&[]);
    let refused = client
        .request_value(
            lsp::HoverRequest::METHOD.as_str(),
            Client::position("main.src", 0, 0),
        )
        .expect_err("not initialized");

    assert_eq!(refused.code, ErrorCodes::ServerNotInitialized);
}

#[test]
fn should_refuse_to_initialize_in_an_unknown_dialect() {
    let mut client = Client::new(&[]);

    client.options.dialect = Some("lua".to_owned());

    let refused = client.try_initialize().expect_err("an unknown dialect");

    assert_eq!(
        (refused.code, refused.message.as_str()),
        (ErrorCodes::InvalidParams, "Unknown dialect: \"lua\"")
    );
}

#[test]
fn should_announce_its_capabilities_on_initialize() {
    let mut client = client_refused_before_initialize();
    let result = client.initialize();
    let capabilities = result.capabilities;

    assert!(capabilities.hover_provider.is_some());
    assert!(capabilities.rename_provider.is_some());
    assert_eq!(
        capabilities.text_document_sync,
        Some(lsp::TextDocumentSync::Options(
            lsp::TextDocumentSyncOptions {
                open_close: Some(true),
                change: Some(lsp::TextDocumentSyncKind::Incremental),
                ..lsp::TextDocumentSyncOptions::default()
            }
        ))
    );
    assert_eq!(
        client.requests,
        [
            lsp::ConfigurationRequest::METHOD.as_str(),
            FileSearch::METHOD.as_str()
        ]
    );
}

#[test]
fn should_respond_to_an_unknown_method_with_method_not_found() {
    let mut client = client_refused_before_initialize();

    client.initialize();

    let unknown = client
        .request_value("textDocument/unknown", json!({}))
        .expect_err("unknown");

    assert_eq!(unknown.code, ErrorCodes::MethodNotFound);
}

#[test]
fn should_not_exit_on_shutdown() {
    let mut client = client_asked_an_unknown_method();

    client
        .request_value(
            lsp::ShutdownRequest::METHOD.as_str(),
            serde_json::Value::Null,
        )
        .expect("shuts down");
    assert_eq!(client.server.exit_code(), None);
}

#[test]
fn should_exit_with_zero_after_shutdown() {
    let mut client = client_shut_down();

    client.notify_value("exit", serde_json::Value::Null);
    assert_eq!(client.server.exit_code(), Some(0));
}

#[test]
fn should_exit_with_one_without_shutdown() {
    let mut abrupt = Client::started(&[]);

    abrupt.notify_value("exit", serde_json::Value::Null);
    assert_eq!(abrupt.server.exit_code(), Some(1), "exit without shutdown");
}
