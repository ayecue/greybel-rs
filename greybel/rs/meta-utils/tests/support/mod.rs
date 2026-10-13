//! The helpers of meta-utils's integration tests.

#![allow(dead_code)]

pub mod cases;

use std::fmt::{Debug, Display};

use meta_utils::{Container, LocalizedDefinition, Variation};

pub fn signature(definitions: &str) -> String {
    format!(r#"{{"type": "thing", "extends": "map", "definitions": {{{definitions}}}}}"#)
}

pub fn assert_rejected<T: Debug, E: Display>(result: Result<T, E>, message: &str) {
    let error = result.expect_err("the payload is invalid");

    assert!(
        error.to_string().contains(message),
        "expected an error containing {message:?}, got: {error}"
    );
}

pub fn hidden_thing() -> Container {
    let mut container = Container::new();

    container
        .add_signature_json(&signature(r#""size": {"type": "number"}"#))
        .expect("valid payload");
    container
        .add_signature_json(r#"{"type": "thing", "hidden": true, "definitions": {}}"#)
        .expect("valid payload");

    container
}

pub fn function(name: &str) -> String {
    format!(r#""{name}": {{"id": "{name}", "type": "function", "returns": ["number"]}}"#)
}

pub fn add(container: &mut Container, type_name: &str, extends: Option<&str>, names: &[&str]) {
    let definitions: Vec<String> = names.iter().map(|name| function(name)).collect();
    let extends = extends.map_or(String::new(), |parent| format!(r#""extends": "{parent}","#));

    container
        .add_signature_json(&format!(
            r#"{{"type": "{type_name}", {extends} "definitions": {{{}}}}}"#,
            definitions.join(",")
        ))
        .expect("valid payload");
}

pub fn origin(found: Option<LocalizedDefinition<'_>>) -> Option<&str> {
    found.map(|found| found.origin())
}

pub fn inheriting_container() -> Container {
    let mut container = Container::new();

    add(&mut container, "map", None, &["len", "keys"]);
    add(&mut container, "thing", Some("map"), &["size", "len"]);
    add(&mut container, "secret", Some("map"), &["hidden_only"]);
    container
        .add_signature_json(r#"{"type": "secret", "hidden": true, "definitions": {}}"#)
        .expect("valid payload");
    add(&mut container, "child", Some("secret"), &["own"]);
    container
        .add_descriptions_json("en", "map", r#"{"keys": {"description": "The keys."}}"#)
        .expect("valid payload");

    container
}

pub fn thing_without_size() -> Container {
    let mut container = Container::new();

    add(&mut container, "thing", Some("map"), &[]);

    container
}

pub fn thing_with_size() -> Container {
    let mut container = thing_without_size();

    add(&mut container, "thing", None, &["size"]);

    container
}

pub fn thing_excluded() -> Container {
    let mut container = thing_with_size();

    container.exclude_from_search("thing");

    container
}

pub fn thing_included_again() -> Container {
    let mut container = thing_excluded();

    container.include_in_search("thing");

    container
}

pub fn other_described() -> Container {
    let mut container = thing_included_again();

    container
        .add_descriptions_json("en", "other", r#"{"$meta": {"description": "Other."}}"#)
        .expect("valid payload");

    container
}

macro_rules! mock {
    ($kind:literal, $name:literal) => {
        include_str!(concat!(
            "../../../meta-utils/tests/fixtures/mocks/",
            $kind,
            "/",
            $name,
            ".json"
        ))
    };
}

/// The container `tests/container.test.js` builds.
pub fn container() -> Container {
    let mut container = Container::new();

    container.add_variations(
        "test.variation",
        [
            Variation::from("hello world"),
            Variation::from(1234.0),
            Variation::from(1337.0),
        ],
    );
    for json in [
        mock!("signatures", "any"),
        mock!("signatures", "general"),
        mock!("signatures", "string"),
        mock!("signatures", "list"),
        mock!("signatures", "map"),
        mock!("signatures", "sub-string"),
        mock!("signatures", "not-searchable"),
    ] {
        container
            .add_signature_json(json)
            .expect("mock signature is valid");
    }

    container.exclude_from_search("not-searchable");
    for (type_name, json) in [
        ("general", mock!("descriptions/en", "general")),
        ("string", mock!("descriptions/en", "string")),
        ("sub-string", mock!("descriptions/en", "sub-string")),
    ] {
        container
            .add_descriptions_json("en", type_name, json)
            .expect("mock descriptions are valid");
    }

    container
}
