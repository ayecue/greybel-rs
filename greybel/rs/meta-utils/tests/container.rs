//! Payload validation, and where the container deliberately differs from TS (see PORTING.md).

use meta_utils::{ArgumentDefault, Container};

use crate::support::{assert_rejected, hidden_thing, signature};

#[test]
fn should_reject_invalid_payloads() {
    let cases = [
        (
            signature(r#""run": {"type": "function", "returns": []}"#),
            "a function needs an `id`",
        ),
        (
            signature(r#""run": {"id": "run", "type": "function"}"#),
            "a function needs `returns`",
        ),
        (
            signature(r#""size": {"type": "number", "id": "size"}"#),
            "`id` is only allowed on functions",
        ),
        (
            signature(
                r#""run": {"id": "run", "type": "function", "returns": [],
                "arguments": [{"label": "a", "type": "number", "types": ["string"]}]}"#,
            ),
            "argument `a` needs either `type` or `types`",
        ),
        (
            signature(
                r#""run": {"id": "run", "type": "function", "returns": [],
                "arguments": [{"label": "a", "type": "number", "default": {"type": "number", "value": "0"}}]}"#,
            ),
            "invalid payload",
        ),
        (
            signature(r#""size": {"type": "number", "colour": "red"}"#),
            "unknown field `colour`",
        ),
        (
            signature(r#""size": {"type": "list<number"}"#),
            "invalid type `list<number`: expected `>` at 11",
        ),
        (
            signature(r#""size": {"type": {"type": "number", "valueType": "string"}}"#),
            "`valueType` does not apply to `number`",
        ),
        (
            signature(r#""size": {"type": {"type": "list", "keyType": "string"}}"#),
            "`keyType` does not apply to `list`",
        ),
        (
            signature(r#""size": {"type": {"type": "list<string>"}}"#),
            "invalid type name `list<string>`",
        ),
        (
            r#"{"type": "thing", "definitions": {}, "parent": "map"}"#.to_owned(),
            "unknown field `parent`",
        ),
    ];

    let outcomes =
        cases.map(|(json, message)| (Container::new().add_signature_json(&json), message));

    for (result, message) in outcomes {
        assert_rejected(result, message);
    }
}

#[test]
fn should_read_arguments_and_defaults() {
    let mut container = Container::new();

    container
        .add_signature_json(&signature(
            r#""run": {"id": "run", "type": "function", "returns": ["list", "map<string, number>"], "arguments": [
                {"label": "count", "type": "number", "default": {"type": "number", "value": 2}},
                {"label": "names", "types": [{"type": "list", "valueType": "string"}, "null"], "opt": true}
            ]}"#,
        ))
        .expect("valid payload");

    let definition = container
        .definition(&["thing"], "run", "en")
        .expect("defined");

    let function = definition.definition().as_function().expect("a function");
    let returns: Vec<String> = function.returns().iter().map(ToString::to_string).collect();

    assert_eq!(returns, ["list<any>", "map<string,number>"]);

    let count = function.argument("count").expect("an argument");

    assert_eq!(count.default(), Some(&ArgumentDefault::Number(2.0)));
    assert!(!count.is_optional());

    let names = &function.arguments()[1];

    assert!(names.is_optional());

    let types: Vec<String> = names.types().iter().map(ToString::to_string).collect();

    assert_eq!(types, ["list<string>", "null"]);
}

#[test]
fn should_keep_extends_when_a_payload_hides_the_type() {
    let container = hidden_thing();

    let thing = container.signature("thing").expect("added");

    assert!(thing.is_hidden());
    assert_eq!(thing.extends(), Some("map"));
}

#[test]
fn should_find_no_definition_on_a_hidden_type() {
    let container = hidden_thing();

    let definition = container.definition(&["thing"], "size", "en");

    assert!(definition.is_none());
}

#[test]
fn should_not_list_a_hidden_type_as_visible() {
    let container = hidden_thing();

    let visible = container.visible_types().any(|name| name == "thing");

    assert!(!visible);
}

#[test]
fn should_copy_the_type_description_and_descriptions() {
    let mut container = Container::new();

    container
        .add_descriptions_json("en", "thing", r#"{"$meta": {"description": "A thing."}}"#)
        .expect("valid payload");
    container
        .add_descriptions_json("en", "thing", r#"{"size": {"description": "Its size."}}"#)
        .expect("valid payload");

    let copy = container.clone();

    let descriptions = copy
        .signature("thing")
        .and_then(|thing| thing.descriptions("en"))
        .expect("copied");

    assert_eq!(
        descriptions.type_description().map(|own| own.text()),
        Some("A thing.")
    );
    assert_eq!(descriptions.members()["size"].text(), "Its size.");
}

#[test]
fn should_read_type_parameters_and_mutations() {
    let mut container = Container::new();

    container
        .add_signature_json(
            r#"{"type": "bag", "typeParameters": ["K", "V"], "definitions": {
                "add": {"id": "add", "type": "function", "mutates": {"V": "number"},
                    "arguments": [{"label": "key", "type": "K"}], "returns": ["self"]},
                "fill": {"id": "fill", "type": "function", "mutates": true, "returns": ["self"]},
                "keys": {"id": "keys", "type": "function", "mutates": false,
                    "returns": [{"type": "list", "valueType": "K"}]}
            }}"#,
        )
        .expect("a valid payload");

    let bag = container.signature("bag").expect("added");

    assert_eq!(bag.type_parameters(), ["K", "V"]);

    let function = |name: &str| {
        bag.definitions()[name]
            .as_function()
            .expect("a function")
            .mutation()
    };
    let fixed: Vec<(String, String)> = function("add")
        .expect("mutates")
        .fixed_types()
        .iter()
        .map(|(name, meta)| (name.clone(), meta.to_string()))
        .collect();

    assert_eq!(fixed, [("V".to_owned(), "number".to_owned())]);
    assert!(function("fill").expect("mutates").fixed_types().is_empty());
    assert!(function("keys").is_none());
}

#[test]
fn should_reject_mutates_on_a_property() {
    let json = signature(r#""size": {"type": "number", "mutates": true}"#);

    let result = Container::new().add_signature_json(&json);

    assert_rejected(result, "`mutates` is only allowed on functions");
}
