use duplex_protocol::{Definition, Payload, TagList, TagPlace, TypeKind, Value};

use crate::support::tags::DEFINITION;
use crate::support::{definition, error_message, protocol, struct_fields};

#[test]
fn should_read_the_tag_declarations() {
    let source = DEFINITION;

    let definition = definition(source);

    let declarations: Vec<(&str, bool, Vec<TagPlace>, bool)> = definition
        .tags()
        .iter()
        .map(|declaration| {
            (
                declaration.name.as_str(),
                declaration.ty.is_some(),
                declaration.places.clone(),
                declaration.repeatable,
            )
        })
        .collect();

    assert_eq!(
        declarations,
        [
            (
                "origin",
                true,
                vec![TagPlace::Protocol, TagPlace::Message],
                false
            ),
            ("game_id", true, vec![TagPlace::Message], false),
            ("list", true, vec![TagPlace::Field], false),
            ("window", false, vec![TagPlace::Field], false),
            (
                "alias",
                true,
                vec![TagPlace::Field, TagPlace::Variant, TagPlace::Member],
                true
            ),
            (
                "kind",
                true,
                vec![TagPlace::Struct, TagPlace::Union, TagPlace::Enumeration],
                false
            ),
        ]
    );
}

#[test]
fn should_list_several_tags_of_an_item_in_their_order_with_their_values() {
    let source = DEFINITION;

    let definition = definition(source);

    let protocol = protocol(&definition);
    let scan = &protocol.messages()[0];
    let values = |tags: &TagList| -> Vec<(String, Option<Value>)> {
        tags.all()
            .iter()
            .map(|tag| (tag.name.clone(), definition.tag_value(tag)))
            .collect()
    };

    assert_eq!(
        values(&protocol.tags),
        [("origin".to_owned(), Some(Value::Text("game".to_owned())))]
    );
    assert_eq!(
        values(&scan.tags),
        [
            ("game_id".to_owned(), Some(Value::Integer(34))),
            (
                "origin".to_owned(),
                Some(Value::Text("extension".to_owned()))
            ),
        ]
    );
    assert_eq!(
        values(&scan.fields[1].tags),
        [
            ("list".to_owned(), Some(Value::Text("intValues".to_owned()))),
            ("window".to_owned(), None),
        ]
    );
}

#[test]
fn should_put_a_repeated_tag_on_an_item_more_than_once() {
    let source = DEFINITION;

    let definition = definition(source);

    let aliases: Vec<Option<Value>> = struct_fields(&definition, "Point")[0]
        .tags
        .find_all("alias")
        .map(|tag| definition.tag_value(tag))
        .collect();

    assert_eq!(
        aliases,
        [
            Some(Value::Text("x".to_owned())),
            Some(Value::Text("across".to_owned()))
        ]
    );
}

#[test]
fn should_put_tags_on_types_variants_members_and_their_fields() {
    let source = DEFINITION;

    let definition = definition(source);

    let shape = definition.declaration("Shape").expect("Shape is declared");
    let kind = definition.declaration("Kind").expect("Kind is declared");
    let TypeKind::Union(variants) = &shape.kind else {
        panic!("Shape is a union");
    };
    let TypeKind::Enumeration(members) = &kind.kind else {
        panic!("Kind is an enumeration");
    };
    let Payload::Fields(line) = &variants[1].payload else {
        panic!("Line has fields");
    };

    assert_eq!(
        definition.tag_value(&shape.tags.all()[0]),
        Some(Value::Member {
            name: "Sum".to_owned(),
            value: 1
        })
    );
    assert!(variants[0].tags.has("alias"));
    assert!(line[0].tags.has("window"));
    assert!(members[0].tags.has("alias"));
    assert!(!members[1].tags.has("alias"));
}

#[test]
fn should_exclude_tags_and_their_declarations_from_the_fingerprint() {
    let untagged = definition("struct S { a: u8 }").fingerprint();

    let tagged = definition(
        "tag name: string on field;\ntag kind on struct;\n@kind\nstruct S { @name(\"a\") a: u8 }",
    )
    .fingerprint();

    assert_eq!(tagged, untagged);
}

#[test]
fn should_reject_an_undeclared_tag() {
    let source = "struct S { @window a: u8 }";

    let message = error_message(Definition::parse(source));

    assert_eq!(
        message,
        "1:12: the tag `window` isn't declared: declare it as `tag window: Type on field;`"
    );
}

#[test]
fn should_reject_a_tag_put_where_it_is_not_declared_for() {
    let source = "tag window on field;\n@window\nstruct S { a: u8 }";

    let message = error_message(Definition::parse(source));

    assert_eq!(
        message,
        "2:1: the tag `window` is put on `field`, not on a `struct`"
    );
}

#[test]
fn should_reject_a_tag_put_on_an_item_twice_unless_it_is_repeated() {
    let source = "tag name: string on field;\nstruct S { @name(\"a\") @name(\"b\") a: u8 }";

    let message = error_message(Definition::parse(source));

    assert_eq!(
        message,
        "2:23: the tag `name` is put on an item once, unless it is declared `repeated`"
    );
}

#[test]
fn should_reject_a_value_of_a_flag() {
    let source = "tag window on field;\nstruct S { @window(1) a: u8 }";

    let message = error_message(Definition::parse(source));

    assert_eq!(
        message,
        "2:12: the tag `window` is a flag: it takes no value"
    );
}

#[test]
fn should_reject_a_tag_without_the_value_it_takes() {
    let source = "tag id: u32 on field;\nstruct S { @id a: u8 }";

    let message = error_message(Definition::parse(source));

    assert_eq!(message, "2:12: the tag `id` takes a value of u32");
}

#[test]
fn should_reject_a_value_of_another_type_than_the_tag_s() {
    let source = "tag id: u32 on field;\nstruct S { @id(\"one\") a: u8 }";

    let message = error_message(Definition::parse(source));

    assert_eq!(message, "2:12: the tag `id`'s value isn't a value of u32");
}

#[test]
fn should_reject_a_tag_of_a_type_without_literals() {
    let source = "tag point: S on field;\nstruct S { a: u8 }";

    let message = error_message(Definition::parse(source));

    assert_eq!(
        message,
        "1:1: the tag `point` is of S; a tag's value is a boolean, character, text, number or member of an enumeration"
    );
}

#[test]
fn should_reject_a_tag_declared_twice() {
    let source = "tag id on field;\ntag id on message;";

    let message = error_message(Definition::parse(source));

    assert_eq!(message, "2:1: the tag `id` is declared twice");
}

#[test]
fn should_reject_a_tag_named_in_another_case_than_snake_case() {
    let source = "tag gameId on field;";

    let message = error_message(Definition::parse(source));

    assert_eq!(message, "1:1: `gameId` is a tag's name in snake_case");
}

#[test]
fn should_reject_a_tag_put_on_an_unknown_place() {
    let source = "tag id on peer;";

    let message = error_message(Definition::parse(source));

    assert_eq!(
        message,
        "1:11: a tag is put on `protocol`, `message`, `field`, `struct`, `union`, `variant`, `enum`, `member`, not `peer`"
    );
}

#[test]
fn should_reject_tags_on_a_constant_or_a_tag() {
    let sources = [
        "tag id on field;\n@id\nconst A: u8 = 1;",
        "tag id on field;\n@id\ntag other on field;",
    ];

    let messages = sources.map(|source| error_message(Definition::parse(source)));

    assert_eq!(
        messages,
        ["3:1: a `const` takes no tags", "3:1: a `tag` takes no tags"]
    );
}

#[test]
fn should_reject_tags_on_a_stream() {
    let source = "use core 1.0;\ntag id on field;\nprotocol P = \"p\" { client C {} provider D { message U { @id items: Stream<u8> } -> bool } }";

    let message = error_message(Definition::parse(source));

    assert_eq!(message, "3:61: a stream takes no tags");
}
