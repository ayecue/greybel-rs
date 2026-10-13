//! Tests initial client settings and subsequent configuration changes.

use greybel_lsp::types::{self as lsp};
use serde_json::json;

use crate::support::{Client, client_with_features_off};

#[test]
fn should_switch_diagnostics_off_by_a_setting() {
    let mut client = client_with_features_off();

    client.open("main.src", "print missing\n");
    assert!(client.codes("main.src").is_empty());
}

#[test]
fn should_switch_hover_off_by_a_setting() {
    let mut client = client_with_features_off();

    client.open("main.src", "print missing\n");

    let hover = client.request::<lsp::HoverRequest>(Client::position("main.src", 0, 7));

    assert!(hover.is_none());
}

#[test]
fn should_take_the_file_extensions_from_the_settings() {
    let client = client_with_features_off();

    let extensions = &client.server.settings().file_extensions;

    assert_eq!(*extensions, ["src"]);
}

#[test]
fn should_switch_diagnostics_on_by_a_configuration_change() {
    let mut client = client_with_features_off();

    client.open("main.src", "print missing\n");
    client.request::<lsp::HoverRequest>(Client::position("main.src", 0, 7));
    client.notify_value(
        "workspace/didChangeConfiguration",
        json!({ "settings": { "greybel": { "diagnostic": true } } }),
    );
    assert_eq!(client.codes("main.src"), ["unresolved-reference"]);
}
