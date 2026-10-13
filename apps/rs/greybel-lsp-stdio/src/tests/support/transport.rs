use greybel_lsp::{Message, Notification};
use serde_json::json;

use super::super::Transport;

pub(super) fn initialized() -> Message {
    Message::Notification(Notification {
        method: "initialized".to_owned(),
        params: json!({}),
    })
}

/// Serializes `message` with LSP framing and prefixes a `content-type` header.
pub(super) fn framed_after_a_content_type(message: &Message) -> Vec<u8> {
    let mut written = Vec::new();

    Transport::new(&b""[..], &mut written)
        .write(message)
        .expect("writes");

    [
        b"content-type: application/vscode-jsonrpc; charset=utf-8\r\n".to_vec(),
        written,
    ]
    .concat()
}
