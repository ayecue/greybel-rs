#![allow(dead_code)]

use serde_json::Value;

pub fn load_string_fixtures() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/string.json");

    serde_json::from_str(&std::fs::read_to_string(path).expect("the fixture")).expect("JSON")
}

pub fn load_collation_fixtures() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/collation.json");

    serde_json::from_str(&std::fs::read_to_string(path).expect("the fixture")).expect("JSON")
}

pub fn rows<'v>(fixtures: &'v Value, name: &str) -> &'v [Value] {
    fixtures[name].as_array().expect("a list of rows")
}

pub fn text(value: &Value) -> &str {
    value.as_str().expect("a string")
}

pub fn number(value: &Value) -> i64 {
    value.as_i64().expect("a number")
}

pub fn position(found: Option<usize>) -> i64 {
    found.map_or(-1, |at| i64::try_from(at).expect("small"))
}

pub fn units(value: &Value) -> Vec<u16> {
    value
        .as_array()
        .expect("units")
        .iter()
        .map(|unit| u16::try_from(unit.as_u64().expect("a unit")).expect("a UTF-16 unit"))
        .collect()
}

pub fn check(differences: &[String]) {
    assert!(
        differences.is_empty(),
        "{} differences from the game:\n{}",
        differences.len(),
        differences
            .iter()
            .take(40)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

pub fn check_of(differences: &[String], total: usize) {
    assert!(
        differences.is_empty(),
        "{} of {total} differ from the game:\n{}",
        differences.len(),
        differences
            .iter()
            .take(40)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
