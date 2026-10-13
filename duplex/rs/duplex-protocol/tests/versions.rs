use duplex_protocol::{Role, Status, TypeKind, Version, VersionHistory};
use duplex_test_support::TempFolder;

use crate::support::plan::struct_fields_plan;
use crate::support::versions::{PROTOCOL, field_since};
use crate::support::{error_message, plan, protocol, version_history};

#[test]
fn should_give_fields_added_by_a_minor_its_minor() {
    let sources = [
        format!("{PROTOCOL}struct Point {{ x: f64 }}"),
        format!("{PROTOCOL}struct Point {{ x: f64, y?: f64 }}"),
        format!("{PROTOCOL}struct Point {{ x: f64, y?: f64, z?: f64 }}"),
    ];

    let versions = version_history(&[
        (1, 0, &sources[0]),
        (1, 1, &sources[1]),
        (1, 2, &sources[2]),
    ])
    .expect("the comparison accepts fields appended by minors");

    assert_eq!(field_since(&versions, 1, "Point"), [0, 1, 2]);
}

#[test]
fn should_plan_the_fields_of_each_minor_in_a_block() {
    let versions = version_history(&[
        (1, 0, "struct Point { x: f64, shown: bool }"),
        (
            1,
            1,
            "struct Point { x: f64, shown: bool, y?: f64, z?: bool }",
        ),
    ])
    .expect("the comparison accepts optional fields appended by a minor");

    let plan = plan(&versions);

    let blocks: Vec<(u16, usize, Vec<u8>)> = struct_fields_plan(&plan, 0)
        .blocks
        .iter()
        .map(|block| (block.since, block.fields.len(), block.flags.clone()))
        .collect();

    assert_eq!(blocks, [(0, 2, vec![0b1]), (1, 2, vec![0b111])]);
}

#[test]
fn should_follow_renamed_types_and_fields_by_their_position() {
    let older = format!("{PROTOCOL}struct Point {{ x: f64 }}");
    let newer = "use core 1.0;\nprotocol P = \"p\" { client C { message Note { spot: Spot } } provider D {} }\nstruct Spot { across: f64, up?: f64 }";

    let versions = version_history(&[(1, 0, &older), (1, 1, newer)])
        .expect("the comparison accepts renamed types and fields");

    assert_eq!(field_since(&versions, 1, "Spot"), [0, 1]);
}

#[test]
fn should_give_messages_variants_and_members_added_by_a_minor_its_minor() {
    let sources = [
        (
            1,
            0,
            "use core 1.0;\nprotocol P = \"p\" { client C { message A { u: U, e: E } } provider D { message B {} } }\nunion U { X }\nenum E { One }",
        ),
        (
            1,
            1,
            "use core 1.0;\nprotocol P = \"p\" { client C { message A { u: U, e: E } message A2 {} } provider D { message B {} } }\nunion U { X, Y(string) }\nenum E { One, Two }",
        ),
    ];

    let versions = version_history(&sources)
        .expect("the comparison accepts appended messages, variants, and members");

    let latest = versions
        .major(1)
        .expect("the versions have major 1")
        .latest();
    let (definition, additions) = (&latest.definition, &latest.additions);
    let messages: Vec<(&str, u16)> = protocol(definition)
        .messages()
        .iter()
        .map(|message| (message.name.as_str(), additions.message(message)))
        .collect();

    assert_eq!(messages, [("A", 0), ("A2", 1), ("B", 0)]);

    let TypeKind::Union(variants) = &definition
        .declaration("U")
        .expect("the latest definition declares `U`")
        .kind
    else {
        panic!("a union");
    };

    assert_eq!(
        variants
            .iter()
            .map(|variant| additions.variant(variant))
            .collect::<Vec<_>>(),
        [0, 1]
    );

    let TypeKind::Enumeration(members) = &definition
        .declaration("E")
        .expect("the latest definition declares `E`")
        .kind
    else {
        panic!("an enumeration");
    };

    assert_eq!(
        members
            .iter()
            .map(|member| additions.member(member))
            .collect::<Vec<_>>(),
        [0, 1]
    );
}

#[test]
fn should_let_a_minor_add_messages_to_either_peer() {
    let sources = [
        (
            1,
            0,
            "use core 1.0;\nprotocol P = \"p\" { client C { message A {} } provider D { message B {} } }",
        ),
        (
            1,
            1,
            "use core 1.0;\nprotocol P = \"p\" { client C { message A {} message A2 {} } provider D { message B {} message B2 {} } }",
        ),
    ];

    let versions = version_history(&sources)
        .expect("the comparison accepts messages appended for either peer");

    let latest = versions
        .latest()
        .expect("the versions have a latest version");
    let ids: Vec<(&str, u32, Role)> = protocol(&latest.definition)
        .messages()
        .iter()
        .map(|message| (message.name.as_str(), message.id, message.receiver))
        .collect();

    assert_eq!(
        ids,
        [
            ("A", 0, Role::Client),
            ("A2", 1, Role::Client),
            ("B", 0, Role::Provider),
            ("B2", 1, Role::Provider),
        ]
    );
}

#[test]
fn should_reject_a_minor_adding_a_required_field() {
    let sources = [
        (1, 0, "struct S { a: u8 }"),
        (1, 1, "struct S { a: u8, b: u8 }"),
    ];

    let message = error_message(version_history(&sources));

    assert_eq!(
        message,
        "test/1.1.protocol:1:19: adds `b`, which is neither optional nor has a default; a minor version adds fields older peers can omit; only a new major, 2.0, may"
    );
}

#[test]
fn should_let_a_minor_add_a_field_with_a_default() {
    let sources = [
        (1, 0, "struct S { a: u8 }"),
        (1, 1, "struct S { a: u8, b: u8 = 3 }"),
    ];

    let versions =
        version_history(&sources).expect("the comparison accepts an appended field with a default");

    assert_eq!(field_since(&versions, 1, "S"), [0, 1]);
}

#[test]
fn should_reject_a_minor_changing_a_default() {
    let newer = [
        "struct S { a: u8 = 4 }",
        "struct S { a: u8 }",
        "const A: u8 = 4;\nstruct S { a: u8 = A }",
    ];

    let messages = newer.map(|newer| {
        error_message(version_history(&[
            (1, 0, "const A: u8 = 3;\nstruct S { a: u8 = A }"),
            (1, 1, newer),
        ]))
    });

    assert!(
        messages
            .iter()
            .all(|message| message
                .contains("changes the default of `a`; only a new major, 2.0, may")),
        "{messages:?}"
    );
}

#[test]
fn should_let_a_minor_change_a_constant_no_default_names() {
    let sources = [
        (1, 0, "const A: u8 = 3;\nstruct S { a: u8 }"),
        (1, 1, "const A: u8 = 4;\nstruct S { a: u8 }"),
    ];

    let result = version_history(&sources);

    assert!(result.is_ok(), "{:?}", result.err());
}

#[test]
fn should_reject_a_minor_breaking_the_layout() {
    let changes = [
        "struct S { b?: u8, a: u8 }",
        "struct S { a: u16 }",
        "struct S { a?: u8 }",
        "struct S {}",
        "union S { A }",
    ];

    let messages = changes.map(|changed| {
        error_message(version_history(&[
            (1, 0, "struct S { a: u8 }"),
            (1, 1, changed),
        ]))
    });

    assert_eq!(
        messages.map(|message| message.split(": ").nth(1).map(str::to_owned)),
        [
            "changes whether `b` is optional; only a new major, 2.0, may",
            "changes the type of `a`; only a new major, 2.0, may",
            "changes whether `a` is optional; only a new major, 2.0, may",
            "removes the field `a`; only a new major, 2.0, may",
            "changes whether `S` is a struct, a union or an enumeration; only a new major, 2.0, may",
        ]
        .map(|message| Some(message.to_owned()))
    );
}

#[test]
fn should_reject_a_minor_changing_how_a_message_streams() {
    let protocol = |message: &str| {
        format!(
            "use core 1.0;\nprotocol P = \"p\" {{ client C {{ {message} }} provider D {{}} }}\nstruct S {{}}"
        )
    };
    let old = protocol("message R { items: Stream<u8> } -> Stream(S, _)<u8>");
    let changes = [
        "message R { items: Stream<u8> } -> u8",
        "message R { items: Stream<u8> } -> Stream<u8>",
        "message R { items: Stream(_, S)<u8> } -> Stream(S, _)<u8>",
        "message R {} -> Stream(S, _)<u8>",
        "message R { items: Stream<u16> } -> Stream(S, _)<u8>",
    ];

    let messages = changes
        .map(|changed| error_message(version_history(&[(1, 0, &old), (1, 1, &protocol(changed))])));

    assert_eq!(
        messages.map(|message| message.split(": ").nth(1).map(str::to_owned)),
        [
            "changes whether `R` streams its response; only a new major, 2.0, may",
            "changes whether the response of `R` starts with a value; only a new major, 2.0, may",
            "changes whether the stream of `R` ends with a value; only a new major, 2.0, may",
            "changes whether `R` streams; only a new major, 2.0, may",
            "changes the type of the items of the stream of `R`; only a new major, 2.0, may",
        ]
        .map(|message| Some(message.to_owned()))
    );
}

#[test]
fn should_let_a_minor_add_variants_to_the_items_of_a_stream() {
    let protocol = |variants: &str| {
        format!(
            "use core 1.0;\nprotocol P = \"p\" {{ client C {{ message R {{}} -> Stream<Delta> }} provider D {{}} }}\nunion Delta {{ {variants} }}"
        )
    };
    let (older, newer) = (protocol("Text(string)"), protocol("Text(string), Done"));

    let result = version_history(&[(1, 0, &older), (1, 1, &newer)]);

    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn should_reject_a_minor_changing_or_removing_members() {
    let changes = ["enum E { A = 2, B = 1 }", "enum E { A = 1 }"];

    let messages = changes.map(|changed| {
        error_message(version_history(&[
            (1, 0, "enum E { A = 1, B = 2 }"),
            (1, 1, changed),
        ]))
    });

    assert!(
        messages[0].contains("changes the value of `A`"),
        "{messages:?}"
    );
    assert!(
        messages[1].contains("removes the member `B`"),
        "{messages:?}"
    );
}

#[test]
fn should_reject_a_minor_adding_to_what_a_handshake_carries() {
    let sources = [
        (
            1,
            0,
            "use core 1.0;\nprotocol P = \"p\" { open Login; client C {} provider D {} }\nstruct Login { user: string }",
        ),
        (
            1,
            1,
            "use core 1.0;\nprotocol P = \"p\" { open Login; client C {} provider D {} }\nstruct Login { user: string, token?: string }",
        ),
    ];

    let message = error_message(version_history(&sources));

    assert!(
        message.contains("adds `token` to what a handshake carries"),
        "{message}"
    );
}

#[test]
fn should_reject_a_minor_moving_to_another_core_major() {
    let sources = [
        (
            1,
            0,
            "use core 1.0;\nprotocol P = \"p\" { client C {} provider D {} }",
        ),
        (
            1,
            1,
            "use core 2.0;\nprotocol P = \"p\" { client C {} provider D {} }",
        ),
    ];

    let message = error_message(version_history(&sources));

    assert!(
        message.contains("uses the core 2.0, of another major than the previous version's 1.0"),
        "{message}"
    );
}

#[test]
fn should_let_a_new_major_change_anything() {
    let sources = [
        (1, 0, "struct S { a: u8 }"),
        (1, 1, "struct S { a: u8, b?: u8 }"),
        (2, 0, "struct S { b: u16 }"),
    ];

    let versions =
        version_history(&sources).expect("the comparison accepts changes in a new major");

    assert_eq!(field_since(&versions, 1, "S"), [0, 1]);
    assert_eq!(field_since(&versions, 2, "S"), [0]);
}

#[test]
fn should_count_each_major_s_minors_up_from_0() {
    let histories: [&[(u16, u16, &str)]; 3] = [
        &[(1, 1, "struct S {}")],
        &[(1, 0, "struct S {}"), (1, 2, "struct S {}")],
        &[(1, 0, "struct S {}"), (2, 1, "struct S {}")],
    ];

    let messages = histories.map(|sources| error_message(version_history(sources)));

    assert_eq!(
        messages,
        [
            "test: has 1.1 where 1.0 comes",
            "test: has 1.2 where 1.1 comes",
            "test: has 2.1 where 2.0 comes",
        ]
    );
}

#[test]
fn should_change_the_fingerprint_with_the_minor_that_added_a_field() {
    let started = version_history(&[(1, 0, "struct S { a: u8, b?: u8 }")])
        .expect("the single version is accepted");

    let grown = version_history(&[
        (1, 0, "struct S { a: u8 }"),
        (1, 1, "struct S { a: u8, b?: u8 }"),
    ])
    .expect("the comparison accepts an optional field appended by a minor");

    assert_ne!(
        grown
            .latest()
            .expect("the versions have a latest version")
            .fingerprint,
        started
            .latest()
            .expect("the versions have a latest version")
            .fingerprint
    );
}

#[test]
fn should_record_each_version_s_status() {
    let sources = [
        (1, 0, "unsupported;\nstruct S {}"),
        (1, 1, "deprecated \"use 2.0\";\nstruct S {}"),
    ];

    let versions =
        version_history(&sources).expect("the comparison accepts version-specific statuses");

    let major = versions.major(1).expect("the versions have major 1");

    assert_eq!(
        [major.status(0), major.status(1)],
        [
            Some(&Status::Unavailable),
            Some(&Status::Deprecation("use 2.0".to_owned()))
        ]
    );
}

#[test]
fn should_release_the_unreleased_versions_of_a_folder() {
    let folder = TempFolder::new(
        "record",
        [
            ("1.0.protocol", "struct S { a: u8 }"),
            ("1.1.protocol", "struct S { a: u8, b?: u8 }"),
        ],
    );

    let released =
        VersionHistory::release(folder.path()).expect("the folder's versions are released");

    assert_eq!(released, [Version::new(1, 0), Version::new(1, 1)]);
}

#[test]
fn should_name_the_loaded_versions_after_their_folder() {
    let folder = TempFolder::new(
        "named-after",
        [
            ("1.0.protocol", "struct S { a: u8 }"),
            ("1.1.protocol", "struct S { a: u8, b?: u8 }"),
        ],
    );

    VersionHistory::release(folder.path()).expect("the folder's versions are released");

    let loaded = VersionHistory::load(folder.path()).expect("the released versions load");

    assert_eq!(
        loaded.name(),
        folder
            .path()
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
    );
}

#[test]
fn should_release_nothing_once_every_version_is_released() {
    let folder = TempFolder::new(
        "released",
        [
            ("1.0.protocol", "struct S { a: u8 }"),
            ("1.1.protocol", "struct S { a: u8, b?: u8 }"),
        ],
    );

    VersionHistory::release(folder.path()).expect("the folder's versions are released");

    let released =
        VersionHistory::release(folder.path()).expect("the folder's versions are released");

    assert_eq!(released, []);
}

#[test]
fn should_require_every_version_but_the_latest_to_be_released() {
    let folder = TempFolder::new(
        "unreleased",
        [
            ("1.0.protocol", "struct S { a: u8 }"),
            ("1.1.protocol", "struct S { a: u8, b?: u8 }"),
        ],
    );

    let message = VersionHistory::load(folder.path())
        .err()
        .map(|error| error.to_string());

    assert!(
        message.as_deref().is_some_and(|message| message
            .ends_with("1.0 isn't released: release it with `RELEASE_VERSIONS=1 cargo test`")),
        "{message:?}"
    );
}

#[test]
fn should_reject_a_released_version_whose_layout_changed() {
    let folder = TempFolder::new("changed", [("1.0.protocol", "struct S { a: u8 }")]);

    VersionHistory::release(folder.path()).expect("the folder's versions are released");
    std::fs::write(folder.path().join("1.0.protocol"), "struct S { a: u16 }")
        .expect("the changed protocol file is written");

    let message = VersionHistory::load(folder.path())
        .err()
        .map(|error| error.to_string());

    assert!(
        message.as_deref().is_some_and(|message| message.ends_with(
            "1.0 changed its layout after it was released; give the change a new version"
        )),
        "{message:?}"
    );
}

#[test]
fn should_reject_a_file_not_named_by_its_version() {
    let folder = TempFolder::new("named", [("latest.protocol", "struct S {}")]);

    let message = VersionHistory::load(folder.path())
        .err()
        .map(|error| error.to_string());

    assert!(
        message.as_deref().is_some_and(|message| message
            .ends_with("`latest.protocol` isn't named by its version, as `1.0.protocol`")),
        "{message:?}"
    );
}
