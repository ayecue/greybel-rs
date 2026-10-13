use duplex_protocol::{
    Definition, HealthCheckSetting, Length, Literal, NumberKind, Payload, Range, Response,
    ResponseBody, Role, Security, SecurityLevel, Status, Stream, StringRequirementContainer,
    TypeKind, TypeRef, Value, Version,
};

use crate::support::{SHOP, definition, error_message, protocol, protocol_source, struct_fields};

#[test]
fn should_number_the_messages_each_peer_handles_in_their_order() {
    let source = SHOP;

    let definition = definition(source);

    let messages: Vec<(&str, u32, Role)> = protocol(&definition)
        .messages()
        .iter()
        .map(|message| (message.name.as_str(), message.id, message.receiver))
        .collect();

    assert_eq!(
        messages,
        [
            ("Order", 0, Role::Provider),
            ("Cancel", 1, Role::Provider),
            ("Confirm", 0, Role::Client),
        ]
    );
}

#[test]
fn should_find_the_peers_by_their_role() {
    let definition = definition(SHOP);
    let protocol = protocol(&definition);

    let peers = [Role::Client, Role::Provider].map(|role| protocol.peer(role).name.as_str());

    assert_eq!(peers, ["Customer", "Counter"]);
}

#[test]
fn should_read_the_channel_of_a_protocol() {
    let source = SHOP;

    let definition = definition(source);

    assert_eq!(protocol(&definition).channel, "shop");
}

#[test]
fn should_read_fields_with_their_modifiers() {
    let source = SHOP;

    let definition = definition(source);

    let messages = protocol(&definition).messages();
    let order = &messages[0];
    let fields: Vec<(&str, bool, &TypeRef)> = order
        .fields
        .iter()
        .map(|field| (field.name.as_str(), field.optional, &field.ty))
        .collect();

    assert_eq!(
        fields,
        [
            (
                "item",
                false,
                &TypeRef::String(StringRequirementContainer::default())
            ),
            (
                "count",
                false,
                &TypeRef::Number(NumberKind::U32, Range::default())
            ),
            ("gift", true, &TypeRef::Bool),
            (
                "note",
                false,
                &TypeRef::Nullable(Box::new(TypeRef::String(
                    StringRequirementContainer::default()
                )))
            ),
        ]
    );
    assert_eq!(
        order.response,
        Some(Response {
            body: ResponseBody::Value(TypeRef::Declared("Receipt".to_owned())),
            failure: Some(TypeRef::Declared("OutOfStock".to_owned())),
        })
    );
    assert!(messages[2].fields[0].flatten);
}

#[test]
fn should_read_the_payloads_of_variants() {
    let source = SHOP;

    let definition = definition(source);

    let TypeKind::Union(variants) = &definition
        .declaration("Receipt")
        .expect("the definition declares `Receipt`")
        .kind
    else {
        panic!("a union");
    };

    assert!(matches!(&variants[0].payload, Payload::Fields(fields) if fields.len() == 2));
    assert_eq!(
        variants[1].payload,
        Payload::Value(TypeRef::String(StringRequirementContainer::default()))
    );
    assert_eq!(variants[2].payload, Payload::Unit);
}

#[test]
fn should_read_doc_comments_and_drop_other_comments() {
    let source =
        "// A note.\n/// The first line.\n///\n///  Indented.\n//// Not a doc.\nenum E { A }";

    let definition = definition(source);

    assert_eq!(
        definition.types()[0].docs.lines(),
        ["The first line.", "", " Indented."]
    );
}

#[test]
fn should_allow_a_definition_of_types_only() {
    let source = "struct Point { x: f64, y: f64 }";

    let definition = definition(source);

    assert!(definition.protocol().is_none());
    assert_eq!(definition.types().len(), 1);
}

#[test]
fn should_take_flatten_as_a_field_name_without_a_following_name() {
    let source = "struct S { flatten: bool }";

    let definition = definition(source);

    let fields = struct_fields(&definition, "S");

    assert_eq!(fields[0].name, "flatten");
    assert!(!fields[0].flatten);
}

#[test]
fn should_tell_where_the_syntax_breaks() {
    let source = "struct S {\n  a: string\n  b: bool\n}";

    let message = error_message(Definition::parse(source));

    assert_eq!(message, "3:3: expected `}`, found `b`");
}

#[test]
fn should_require_a_client_and_a_provider() {
    let source = protocol_source("client C {}", "");

    let message = error_message(Definition::parse(&source));

    assert_eq!(message, "1:15: the protocol has a client and a provider");
}

#[test]
fn should_reject_a_second_peer_of_a_role() {
    let source = protocol_source("client C {} client D {}", "");

    let message = error_message(Definition::parse(&source));

    assert_eq!(message, "1:46: the protocol has one client");
}

#[test]
fn should_read_what_a_response_fails_with() {
    let source = SHOP;

    let definition = definition(source);

    let failures: Vec<_> = protocol(&definition)
        .messages()
        .iter()
        .map(|message| message.failure())
        .collect();

    assert_eq!(
        failures,
        [
            Some(&TypeRef::Declared("OutOfStock".to_owned())),
            None,
            None
        ]
    );
}

#[test]
fn should_read_a_message_without_a_response_as_a_notification() {
    let source = protocol_source("client C { message N {} } provider D {}", "");

    let definition = definition(&source);

    assert_eq!(protocol(&definition).messages()[0].response, None);
}

#[test]
fn should_read_streamed_responses() {
    let response = |declared: &str| {
        let source = protocol_source(
            &format!("client C {{ message R {{}} -> {declared} }} provider D {{}}"),
            "struct S {}\nstruct E {}\nenum F { A }",
        );

        protocol(&definition(&source)).messages()[0]
            .response
            .clone()
            .expect("the test message has a response")
    };
    let declared = |name: &str| Some(TypeRef::Declared(name.to_owned()));
    let stream = |start, end| {
        ResponseBody::Stream(Stream {
            start,
            end,
            item: TypeRef::Number(NumberKind::U8, Range::default()),
            items: Length::ANY,
        })
    };
    let responses = [
        "Stream<u8>",
        "Stream(S, E)<u8>",
        "Stream(_, E)<u8>",
        "Result<Stream(S, _)<u8>, F>",
    ];

    let streams = responses.map(response);

    assert_eq!(
        streams.map(|response| (response.body, response.failure)),
        [
            (stream(None, None), None),
            (stream(declared("S"), declared("E")), None),
            (stream(None, declared("E")), None),
            (stream(declared("S"), None), declared("F")),
        ]
    );
}

#[test]
fn should_read_the_stream_of_a_message() {
    let source = protocol_source(
        "provider D { message Upload { name: string, chunks: Stream(_, u32)<bytes> } -> bool } client C {}",
        "",
    );

    let definition = definition(&source);

    let upload = &protocol(&definition).messages()[0];

    assert_eq!(upload.fields.len(), 1);

    let stream = upload
        .stream
        .as_ref()
        .expect("the upload message has a stream");

    assert_eq!(stream.name, "chunks");
    assert_eq!(
        stream.stream,
        Stream {
            start: None,
            end: Some(TypeRef::Number(NumberKind::U32, Range::default())),
            item: TypeRef::Bytes(Length::ANY),
            items: Length::ANY,
        }
    );
}

#[test]
fn should_reject_streams_out_of_place() {
    let messages = [
        "message R { items: Stream<u8>, name: string } -> u8",
        "message R { items: Stream(u8, _)<u8> } -> u8",
        "message R { items: Stream<u8> }",
        "message R { items?: Stream<u8> } -> u8",
    ];

    let errors = messages.map(|message| {
        error_message(Definition::parse(&protocol_source(
            &format!("client C {{ {message} }} provider D {{}}"),
            "",
        )))
    });

    assert_eq!(
        errors,
        [
            "1:76: a stream is a message's last field",
            "1:64: a message's stream starts with no value: its other fields are its start",
            "1:45: `R` streams, so it declares a response: what its receiver makes of it",
            "1:57: a stream is neither optional nor flattened",
        ]
    );
}

#[test]
fn should_reject_a_type_named_as_a_keyword_of_responses() {
    let source = "use core 1.0;\nstruct Stream {}";

    let message = error_message(Definition::parse(source));

    assert_eq!(message, "2:1: `Stream` is a keyword of responses");
}

#[test]
fn should_read_the_values_given_to_members() {
    let source = "enum Code { Closed = 1, Timeout = 2 }";

    let definition = definition(source);

    let TypeKind::Enumeration(members) = &definition.types()[0].kind else {
        panic!("an enumeration");
    };
    let values: Vec<_> = members
        .iter()
        .map(|member| (member.name.as_str(), member.value))
        .collect();

    assert_eq!(values, [("Closed", Some(1)), ("Timeout", Some(2))]);
}

#[test]
fn should_read_bytes_and_fixed_width_integers() {
    let source = "struct S { data: bytes, hash: fixed32, seed: fixed64 }";

    let definition = definition(source);

    let tys: Vec<_> = struct_fields(&definition, "S")
        .iter()
        .map(|field| &field.ty)
        .collect();

    assert_eq!(
        tys,
        [
            &TypeRef::Bytes(Length::ANY),
            &TypeRef::Number(NumberKind::Fixed32, Range::default()),
            &TypeRef::Number(NumberKind::Fixed64, Range::default()),
        ]
    );
}

#[test]
fn should_reject_a_value_past_u32() {
    let source = "enum E { A = 4294967296 }";

    let message = error_message(Definition::parse(source));

    assert_eq!(message, "1:14: `4294967296` is a number below 2^32");
}

#[test]
fn should_read_the_core_and_the_status() {
    let source =
        "use core 1.2;\ndeprecated \"use 2.0\";\nprotocol P = \"p\" { client C {} provider D {} }";

    let definition = definition(source);

    assert_eq!(definition.core(), Some(Version::new(1, 2)));
    assert_eq!(
        *definition.status(),
        Status::Deprecation("use 2.0".to_owned())
    );
}

#[test]
fn should_read_an_unsupported_version() {
    let source = "unsupported;\nstruct S {}";

    let definition = definition(source);

    assert_eq!(*definition.status(), Status::Unavailable);
}

#[test]
fn should_use_only_the_core_for_now() {
    let source = "use documents 1.0 as docs;\nstruct S {}";

    let message = error_message(Definition::parse(source));

    assert_eq!(
        message,
        "1:1: `use` names `core` only; `documents` can't be used yet"
    );
}

#[test]
fn should_require_a_protocol_to_use_the_core() {
    let source = "protocol P = \"p\" { client C {} provider D {} }";

    let message = error_message(Definition::parse(source));

    assert_eq!(
        message,
        "1:1: a protocol names the core it builds on, as `use core 1.0;`"
    );
}

#[test]
fn should_read_the_handshake_s_payloads() {
    let source = protocol_source(
        "open Credentials; ack Session; refuse Denial; client C {} provider D {}",
        "struct Credentials { token: string }\nstruct Session { user: string }\nenum Denial { Expired }",
    );

    let definition = definition(&source);

    let handshake = &protocol(&definition).handshake;

    assert_eq!(
        [&handshake.open, &handshake.ack, &handshake.refusal].map(Clone::clone),
        ["Credentials", "Session", "Denial"].map(|name| Some(TypeRef::Declared(name.to_owned())))
    );
}

#[test]
fn should_reject_a_refusal_that_is_no_enumeration_or_union() {
    let source = protocol_source("refuse S; client C {} provider D {}", "struct S {}");

    let message = error_message(Definition::parse(&source));

    assert_eq!(
        message,
        "1:15: a refusal is a declared enumeration or union"
    );
}

#[test]
fn should_reject_a_second_payload_of_a_kind() {
    let source = protocol_source("ack bool; ack bool; client C {} provider D {}", "");

    let message = error_message(Definition::parse(&source));

    assert_eq!(message, "1:44: the protocol declares one `ack` payload");
}

#[test]
fn should_parse_what_security_a_protocol_needs_and_whether_it_is_signed() {
    let securities = ["security encrypted;", "security authenticated signed;"];

    let needs = securities.map(|security| {
        protocol(&definition(&protocol_source(
            &format!("{security} client C {{}} provider D {{}}"),
            "",
        )))
        .security
    });

    assert_eq!(
        needs,
        [
            Some(Security {
                level: SecurityLevel::Encryption,
                signed: false
            }),
            Some(Security {
                level: SecurityLevel::Authentication,
                signed: true
            }),
        ]
    );
}

#[test]
fn should_refuse_signed_as_a_security_level() {
    let source = protocol_source("security signed; client C {} provider D {}", "");

    let parsed = Definition::parse(&source);

    assert!(parsed.is_err());
}

#[test]
fn should_parse_the_name_a_message_had_in_the_major_before() {
    let source = protocol_source(
        "client C {} provider D { message Total renames Sum { a: u8 } }",
        "",
    );

    let definition = definition(&source);

    let message = &protocol(&definition).messages()[0];

    assert_eq!(
        (message.name.as_str(), message.renames.as_deref()),
        ("Total", Some("Sum"))
    );
}

#[test]
fn should_parse_the_flavours_of_a_protocol() {
    let source = protocol_source(
        "flavour stdio { health_check = off; } flavour worker { health_check = { interval = 1000, close_threshold = 5 }; } client C {} provider D {}",
        "",
    );

    let definition = definition(&source);

    let flavours: Vec<_> = protocol(&definition)
        .flavours
        .iter()
        .map(|flavour| (flavour.transport.as_str(), flavour.health_check))
        .collect();

    assert_eq!(
        flavours,
        [
            ("stdio", Some(HealthCheckSetting::Off)),
            (
                "worker",
                Some(HealthCheckSetting::On {
                    interval: Some(1000),
                    unhealthy_threshold: None,
                    close_threshold: Some(5),
                })
            ),
        ]
    );
}

#[test]
fn should_parse_defaults_and_constants() {
    let source =
        "const LIMIT: i32 = -3;\nstruct S { a: i32 = LIMIT, b: f64 = 2.25, c: bool = true }";

    let definition = definition(source);

    let fields = struct_fields(&definition, "S");
    let defaults: Vec<_> = fields.iter().map(|field| field.default.clone()).collect();

    assert_eq!(
        defaults,
        [
            Some(Literal::Name("LIMIT".to_owned())),
            Some(Literal::Number("2.25".to_owned())),
            Some(Literal::Bool(true)),
        ]
    );
    assert_eq!(definition.default_of(&fields[0]), Some(Value::Integer(-3)));
    assert_eq!(definition.constants()[0].name, "LIMIT");
}

#[test]
fn should_read_the_timeouts_of_messages_in_their_units() {
    let source = protocol_source(
        "client C {} provider D { message A {} -> u8 timeout 5s message B {} -> u8 timeout 250ms message E {} -> u8 timeout 2m message F {} -> u8 timeout 40 message G {} -> u8 }",
        "",
    );

    let definition = definition(&source);

    let timeouts: Vec<_> = protocol(&definition)
        .messages()
        .iter()
        .map(|message| message.timeout)
        .collect();

    assert_eq!(
        timeouts,
        [Some(5_000), Some(250), Some(120_000), Some(40), None]
    );
}

#[test]
fn should_read_the_timeout_and_the_most_a_message_takes_of_a_flavour() {
    let source = protocol_source(
        "flavour http { timeout = 30s; max_message = 4MiB; health_check = { interval = 2s }; } client C {} provider D {}",
        "",
    );

    let definition = definition(&source);

    let flavour = &protocol(&definition).flavours[0];

    assert_eq!(
        (flavour.timeout, flavour.max_message, flavour.health_check),
        (
            Some(30_000),
            Some(4 << 20),
            Some(HealthCheckSetting::On {
                interval: Some(2_000),
                unhealthy_threshold: None,
                close_threshold: None,
            })
        )
    );
}

#[test]
fn should_reject_a_timeout_of_a_notification() {
    let source = protocol_source("client C {} provider D { message A {} timeout 5s }", "");

    let message = error_message(Definition::parse(&source));

    assert_eq!(
        message,
        "1:72: `A` is a notification: no response to time out"
    );
}

#[test]
fn should_reject_durations_and_sizes_out_of_range_or_of_other_units() {
    let settings = ["timeout = 5h", "timeout = 0s", "max_message = 8GiB"];

    let messages = settings.map(|setting| {
        error_message(Definition::parse(&protocol_source(
            &format!("flavour http {{ {setting}; }} client C {{}} provider D {{}}"),
            "",
        )))
    });

    assert_eq!(
        messages,
        [
            "1:59: `h` is no unit of a duration: `ms`, `s`, `m`",
            "1:59: `0s` is a duration above 0 and below 2^32",
            "1:63: `8GiB` is a size above 0 and below 2^32",
        ]
    );
}
