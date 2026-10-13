//! Builders for language-client request JSON parameters.

use serde_json::{Value, json};

/// Builds `initialize` parameters for one `project` workspace at `root` with client `capabilities`.
#[must_use]
pub fn initialize_params(root: &str, capabilities: Value) -> Value {
    json!({
        "processId": null,
        "rootUri": null,
        "capabilities": capabilities,
        "workspaceFolders": [{ "uri": root, "name": "project" }]
    })
}

/// Builds version-1 `textDocument/didOpen` parameters for `text` at `uri` in `language`.
#[must_use]
pub fn did_open_params(uri: &str, language: &str, text: &str) -> Value {
    json!({ "textDocument": { "uri": uri, "languageId": language, "version": 1, "text": text } })
}
