//! Tests value requirements and message size limits.

use duplex_protocol::{
    Definition, Length, NumberKind, Protocol, Range, StringRequirementContainer, TypeRef,
};

use crate::support::{definition, error_message, protocol, protocol_source, struct_fields};

#[test]
fn should_read_the_requirements_of_each_type() {
    let source = "struct S { name: string(1..=32, match \"[a-z]+\"), age: u8(13..=120), score: f64(-0.5..=1.0), tags: list<string(..=16)>(..=8), avatar: bytes(..=256KiB), counts: record<u8>(1..), note: string(..=4)? }";

    let definition = definition(source);

    let types: Vec<&TypeRef> = struct_fields(&definition, "S")
        .iter()
        .map(|field| &field.ty)
        .collect();

    assert_eq!(
        types,
        [
            &TypeRef::String(StringRequirementContainer {
                length: Length {
                    min: 1,
                    max: Some(32)
                },
                pattern: Some("[a-z]+".to_owned()),
            }),
            &TypeRef::Number(
                NumberKind::U8,
                Range {
                    min: Some("13".to_owned()),
                    max: Some("120".to_owned())
                }
            ),
            &TypeRef::Number(
                NumberKind::F64,
                Range {
                    min: Some("-0.5".to_owned()),
                    max: Some("1.0".to_owned())
                }
            ),
            &TypeRef::List(
                Box::new(TypeRef::String(StringRequirementContainer {
                    length: Length {
                        min: 0,
                        max: Some(16)
                    },
                    pattern: None,
                })),
                Length {
                    min: 0,
                    max: Some(8)
                }
            ),
            &TypeRef::Bytes(Length {
                min: 0,
                max: Some(256 * 1024)
            }),
            &TypeRef::Record(
                Box::new(TypeRef::Number(NumberKind::U8, Range::default())),
                Length { min: 1, max: None }
            ),
            &TypeRef::Nullable(Box::new(TypeRef::String(StringRequirementContainer {
                length: Length {
                    min: 0,
                    max: Some(4)
                },
                pattern: None,
            }))),
        ]
    );
}

#[test]
fn should_read_limits_and_the_items_of_a_stream() {
    let source = protocol_source(
        "limit 16KiB; provider D { message Upload { chunks: Stream<bytes(..=4KiB)>(..=1000) } -> bool timeout 5s limit 8KiB message Ping {} } client C {}",
        "",
    );

    let definition = definition(&source);

    let protocol = protocol(&definition);
    let [upload, ping] = protocol.messages() else {
        unreachable!("two messages")
    };

    assert_eq!(protocol.limit, Some(16 * 1024));
    assert_eq!((upload.limit, upload.timeout), (Some(8 * 1024), Some(5000)));
    assert_eq!(
        upload
            .stream
            .as_ref()
            .expect("the upload message has a stream")
            .stream
            .items,
        Length {
            min: 0,
            max: Some(1000)
        }
    );
    assert_eq!(
        [protocol.limit_of(upload), protocol.limit_of(ping)],
        [8 * 1024, 16 * 1024]
    );
    assert_eq!(Protocol::DEFAULT_LIMIT, 64 * 1024);
}

#[test]
fn should_reject_a_range_that_isnt_one() {
    let types = [
        "string(1..4)",
        "string(..)",
        "u8(1...2)",
        "list<u8>(1KiB..)",
    ];

    let messages =
        types.map(|ty| error_message(Definition::parse(&format!("struct S {{ a: {ty} }}"))));

    assert_eq!(
        messages,
        [
            "1:22: a range is `least..=most`, `..=most` or `least..`",
            "1:22: a range is `least..=most`, `..=most` or `least..`",
            "1:18: a range is `least..=most`, `..=most` or `least..`",
            "1:24: a count has no unit: `1KiB`",
        ]
    );
}

#[test]
fn should_reject_requirements_that_contradict_themselves() {
    let types = [
        "string(5..=4)",
        "u8(0..=300)",
        "i8(5..=-5)",
        "u32(0.5..)",
        "string(match \"(a\")",
    ];

    let messages =
        types.map(|ty| error_message(Definition::parse(&format!("struct S {{ a: {ty} }}"))));

    assert_eq!(
        messages[0],
        "1:12: the length `5..=4` has a least above its most"
    );
    assert_eq!(
        messages[1],
        "1:12: the bounds of u8 `300` is out of the range of u8"
    );
    assert_eq!(
        messages[2],
        "1:12: the bounds of i8 has a least above its most"
    );
    assert_eq!(
        messages[3],
        "1:12: the bounds of u32 `0.5` isn't an integer"
    );
    assert!(
        messages[4].starts_with("1:12: the pattern `(a` isn't a pattern a table can match"),
        "{}",
        messages[4]
    );
}

#[test]
fn should_reject_defaults_and_constants_that_dont_meet_their_requirements() {
    let sources = [
        "struct S { a: string(..=2) = \"abc\" }",
        "struct S { a: string(match \"[a-z]+\") = \"A\" }",
        "struct S { a: u8(1..=5) = 9 }",
        "const NAME: string(1..) = \"\";",
    ];

    let messages = sources.map(|source| error_message(Definition::parse(source)));

    assert_eq!(
        messages,
        [
            "1:12: the default of `a` is 3 bytes long, out of ..=2",
            "1:12: the default of `a` doesn't match `[a-z]+`",
            "1:12: the default of `a` is out of its bounds",
            "1:1: the constant `NAME` is 0 bytes long, out of 1..",
        ]
    );
}

#[test]
fn should_reject_a_limit_on_what_the_client_sends_nothing_of() {
    let source = protocol_source("provider D {} client C { message Shown {} limit 1KiB }", "");

    let message = error_message(Definition::parse(&source));

    assert_eq!(
        message,
        "1:59: `Shown` is a notification the provider sends: the client sends nothing of it to limit"
    );
}

#[test]
fn should_reject_bounded_fields_past_the_limit_of_their_frame() {
    let sources = [
        protocol_source(
            "provider D { message Big { a: bytes(..=70KiB) } } client C {}",
            "",
        ),
        protocol_source(
            "limit 1KiB; provider D { message Big { a: list<string(..=100)>(..=20) } } client C {}",
            "",
        ),
        protocol_source(
            "provider D { message Up { chunks: Stream<bytes(..=2KiB)> } -> bool limit 1KiB } client C {}",
            "",
        ),
        protocol_source(
            "provider D {} client C { message Get {} -> S limit 1KiB }",
            "struct S { a: string(..=2KiB), b: string }",
        ),
    ];

    let messages = sources.map(|source| error_message(Definition::parse(&source)));

    assert_eq!(
        messages,
        [
            "1:47: the message of `Big` takes up to 71683 bytes, past its limit of 65536: give `Big` a larger `limit`",
            "1:59: the message of `Big` takes up to 2021 bytes, past its limit of 1024: give `Big` a larger `limit`",
            "1:47: an item of the stream of `Up` takes up to 2050 bytes, past its limit of 1024: give `Up` a larger `limit`",
            "1:59: the response of `Get` takes up to 2050 bytes, past its limit of 1024: give `Get` a larger `limit`",
        ]
    );
}

#[test]
fn should_accept_unbounded_fields_beside_bounded_ones_within_the_limit() {
    let source = protocol_source(
        "limit 1KiB; provider D { message Note { title: string(..=100), body: string, tags: list<string> } } client C {}",
        "",
    );

    let result = Definition::parse(&source);

    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn should_exclude_requirements_and_limits_from_the_fingerprint() {
    let plain = definition(&protocol_source(
        "provider D { message Note { title: string, rating: u8, chunks: Stream<bytes> } -> bool } client C {}",
        "",
    ))
    .fingerprint();

    let required = definition(&protocol_source(
        "limit 4KiB; provider D { message Note { title: string(1..=100, match \"[a-z]+\"), rating: u8(1..=5), chunks: Stream<bytes(..=1KiB)>(..=10) } -> bool limit 2KiB } client C {}",
        "",
    ))
    .fingerprint();

    assert_eq!(required, plain);
}
