//! `custom/project` requests.

use greybel_lsp::{ProjectRequest, ProjectResult};
use serde_json::json;

use super::{Client, uri};

/// The project of the document at `path`, with greybel-vs's settings `fallback`.
pub fn project(client: &mut Client, path: &str, fallback: serde_json::Value) -> ProjectResult {
    client.request::<ProjectRequest>(json!({ "uri": uri(path), "fallback": fallback }))
}
