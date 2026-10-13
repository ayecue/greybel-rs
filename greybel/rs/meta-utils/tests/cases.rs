//! Snapshots the TypeScript package's test inputs; differences are listed in `PORTING.md`.

use greybel_test_support::meta::snapshot_parts;
use meta_utils::TypeMeta;
use serde_json::json;
use support::cases::{Case, inputs, outcome};
use support::container;

use crate::support;

#[test]
fn should_give_definitions() {
    let container = container();
    let lookups = [
        ("string", "split"),
        ("sub-string", "myProperty"),
        ("sub-string", "split"),
        ("sub-string", "hasIndex"),
    ];

    let definitions = lookups.map(|(type_name, property)| {
        json!({
            "type": type_name,
            "property": property,
            "definition": container.definition(&[type_name], property, "en"),
        })
    });

    insta::assert_yaml_snapshot!(definitions);
}

#[test]
fn should_parse_type_strings() {
    let inputs = inputs("typeStrings");

    let cases: Vec<Case> = inputs
        .iter()
        .map(|input| Case {
            input,
            output: outcome(input.as_str().expect("a string").parse::<TypeMeta>()),
        })
        .collect();

    insta::assert_yaml_snapshot!(cases);
}

#[test]
fn should_read_type_objects() {
    let inputs = inputs("typeObjects");

    let cases: Vec<Case> = inputs
        .iter()
        .map(|input| Case {
            input,
            output: outcome(serde_json::from_value::<TypeMeta>(input.clone())),
        })
        .collect();

    insta::assert_yaml_snapshot!(cases);
}

#[test]
fn should_dump_the_container() {
    let container = container();

    let parts = snapshot_parts(&container);

    for (name, part) in parts {
        insta::assert_yaml_snapshot!(name, part);
    }
}
