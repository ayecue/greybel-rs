use crate::support::{SHOP, definition, protocol, protocol_source};

#[test]
fn should_give_the_same_definition_the_same_fingerprint() {
    let first = definition(SHOP).fingerprint();

    let second = definition(SHOP).fingerprint();

    assert_eq!(first, second);
}

#[test]
fn should_exclude_docs_and_comments_from_the_fingerprint() {
    let plain = definition("struct Point { x: f64, y: f64 }").fingerprint();
    let documented = "/// A point.\nstruct Point { /// Across.\n x: f64, y: f64 } // The end.";

    let fingerprint_with_docs = definition(documented).fingerprint();

    assert_eq!(fingerprint_with_docs, plain);
}

#[test]
fn should_exclude_the_status_from_the_fingerprint() {
    let current = definition("struct S { a: u8 }").fingerprint();

    let deprecated = definition("deprecated \"gone soon\";\nstruct S { a: u8 }").fingerprint();

    assert_eq!(deprecated, current);
}

#[test]
fn should_exclude_a_protocol_s_names_from_the_fingerprint() {
    let named = |message: &str, field: &str, ty: &str| {
        definition(&protocol_source(
            &format!(
                "client C {{ message {message} {{ {field}: {ty} }} -> {ty} }} provider D {{}}"
            ),
            &format!("struct {ty} {{ value: u8 }}"),
        ))
        .fingerprint()
    };
    let original = named("Request", "body", "Body");

    let renamed = named("Query", "question", "Question");

    assert_eq!(renamed, original);
}

#[test]
fn should_change_the_fingerprint_with_the_layout() {
    let base = definition("struct Point { x: f64, y: f64 }").fingerprint();
    let changes = [
        "struct Point { x: f32, y: f64 }",
        "struct Point { x: f64, y?: f64 }",
        "struct Point { x: f64, y: f64? }",
        "struct Point { x: f64, y: f64, z: f64 }",
        "struct Spot { x: f64, y: f64 }",
    ];

    let changed = changes.map(|source| definition(source).fingerprint());

    assert!(changed.iter().all(|changed| *changed != base));
}

#[test]
fn should_change_the_fingerprint_with_what_a_response_fails_with() {
    let responses = ["bool", "Result<bool, E>", "Result<bool, F>"];

    let fingerprints = responses.map(|response| {
        definition(&protocol_source(
            &format!("client C {{ message R {{}} -> {response} }} provider D {{}}"),
            "enum E { A }\nenum F { A, B }",
        ))
        .fingerprint()
    });

    assert_ne!(fingerprints[0], fingerprints[1]);
    assert_ne!(fingerprints[1], fingerprints[2]);
}

#[test]
fn should_change_the_fingerprint_with_what_streams() {
    let messages = [
        "message R {} -> u8",
        "message R {} -> Stream<u8>",
        "message R {} -> Stream(S, _)<u8>",
        "message R {} -> Stream(_, S)<u8>",
        "message R { items: Stream<u8> } -> u8",
        "message R { items: Stream(_, S)<u8> } -> u8",
    ];

    let fingerprints = messages.map(|message| {
        definition(&protocol_source(
            &format!("client C {{ {message} }} provider D {{}}"),
            "struct S {}",
        ))
        .fingerprint()
    });

    for (index, fingerprint) in fingerprints.iter().enumerate() {
        assert!(
            !fingerprints[..index].contains(fingerprint),
            "{index}: {fingerprints:?}"
        );
    }
}

#[test]
fn should_change_the_fingerprint_with_the_receiver_of_a_message() {
    let bodies = [
        "client C { message N {} } provider D {}",
        "client C {} provider D { message N {} }",
    ];

    let fingerprints = bodies.map(|body| definition(&protocol_source(body, "")).fingerprint());

    assert_ne!(fingerprints[0], fingerprints[1]);
}

#[test]
fn should_change_the_fingerprint_with_the_handshake() {
    let bodies = [
        "client C {} provider D {}",
        "open string; client C {} provider D {}",
        "ack string; client C {} provider D {}",
    ];

    let fingerprints = bodies.map(|body| definition(&protocol_source(body, "")).fingerprint());

    assert_ne!(fingerprints[0], fingerprints[1]);
    assert_ne!(fingerprints[1], fingerprints[2]);
}

#[test]
fn should_hash_the_channel_name_for_its_id() {
    let definition = definition(SHOP);

    let id = protocol(&definition).channel_id();

    assert_eq!(id, 0xa847_e0a9);
}

#[test]
fn should_change_the_fingerprint_with_the_values_of_members() {
    let sources = [
        "enum E { A, B }",
        "enum E { A = 0, B = 1 }",
        "enum E { A = 1, B = 2 }",
    ];

    let fingerprints = sources.map(|source| definition(source).fingerprint());

    assert_eq!(fingerprints[0], fingerprints[1]);
    assert_ne!(fingerprints[1], fingerprints[2]);
}

#[test]
fn should_change_the_fingerprint_with_a_default_but_not_a_constant_it_doesn_t_name() {
    let sources = [
        "struct S { a: u8 }",
        "struct S { a: u8 = 1 }",
        "struct S { a: u8 = 2 }",
        "const A: u8 = 9;\nstruct S { a: u8 }",
    ];

    let [plain, defaulted, other, constant] =
        sources.map(|source| definition(source).fingerprint());

    assert_ne!(plain, defaulted);
    assert_ne!(defaulted, other);
    assert_eq!(plain, constant);
}

#[test]
fn should_exclude_timeouts_and_flavours_from_the_fingerprint() {
    let untimed = definition(&protocol_source(
        "client C {} provider D { message A {} -> u8 }",
        "",
    ))
    .fingerprint();

    let timed = definition(&protocol_source(
        "flavour http { timeout = 30s; max_message = 1MiB; } client C {} provider D { message A {} -> u8 timeout 5s }",
        "",
    ))
    .fingerprint();

    assert_eq!(timed, untimed);
}
