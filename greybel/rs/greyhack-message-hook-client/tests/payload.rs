use duplex_core::Bytes;
use greyhack_message_hook_client::Payload;

#[test]
fn should_round_trip_text() {
    let payload = Bytes::from_text("print 1\nöäü");
    let text = payload.text();

    assert_eq!(text, "print 1\nöäü");
}

#[test]
fn should_use_the_games_raw_and_gzip_modes() {
    let raw = Bytes::from_text("short");

    assert_eq!(raw.0[0], 0);
    assert_eq!(raw.text(), "short");

    let text = "a repetitive source line\n".repeat(100);
    let zipped = Bytes::from_text(&text);

    assert_eq!(zipped.0[0], 1);
    assert_eq!(zipped.text(), text);
}

#[test]
fn should_round_trip_json() {
    let payload = Bytes::from_json(&[1, 2, 3]);
    let json = payload.json::<Vec<i32>>();

    assert_eq!(json, Some(vec![1, 2, 3]));
}

#[test]
fn should_read_a_broken_payload_as_empty_text() {
    let payload = Bytes(b"not gzip".to_vec());
    let text = payload.text();

    assert_eq!(text, "");
}

#[test]
fn should_read_an_empty_payload_as_no_json() {
    let payload = Bytes::default();
    let json = payload.json::<serde_json::Value>();

    assert_eq!(json, None);
}
