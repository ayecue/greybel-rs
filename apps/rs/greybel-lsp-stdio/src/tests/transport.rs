#[path = "support/transport.rs"]
mod support;

use support::{framed_after_a_content_type, initialized};

use super::Transport;

#[test]
fn should_frame_a_written_message_with_its_length() {
    let message = initialized();
    let mut written = Vec::new();

    Transport::new(&b""[..], &mut written)
        .write(&message)
        .expect("writes");

    let text = String::from_utf8(written).expect("utf-8");

    assert!(text.starts_with("Content-Length: "), "{text}");
}

#[test]
fn should_read_a_framed_message_after_other_headers() {
    let message = initialized();
    let input = framed_after_a_content_type(&message);
    let mut transport = Transport::new(&input[..], Vec::new());
    let read = transport.read().expect("reads");

    assert_eq!(read, Some(message));
}

#[test]
fn should_read_nothing_after_the_last_message() {
    let input = framed_after_a_content_type(&initialized());
    let mut transport = Transport::new(&input[..], Vec::new());

    transport.read().expect("reads");

    let read = transport.read().expect("reads");

    assert_eq!(read, None);
}
