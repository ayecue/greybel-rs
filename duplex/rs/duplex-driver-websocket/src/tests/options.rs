use super::WebSocketOptions;

#[test]
fn should_allow_the_origins_it_says() {
    let own = WebSocketOptions::default();
    let other = WebSocketOptions {
        allow_origin: Some("http://pages.test".to_owned()),
        ..WebSocketOptions::default()
    };
    let allowed = [
        own.allows(None, Some("api.test")),
        own.allows(Some("http://api.test"), Some("api.test")),
        own.allows(Some("http://pages.test"), Some("api.test")),
        other.allows(Some("http://pages.test"), Some("api.test")),
        other.allows(Some("http://evil.test"), Some("api.test")),
    ];

    assert_eq!(allowed, [true, true, false, true, false]);
}
