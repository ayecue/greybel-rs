//! Shared helpers for `greybel-io-web` integration tests.

#![allow(dead_code)]

use duplex_test_support::binary::unhex;
use greybel_io::{Key, Side};
use serde_json::Value;

/// Hex message layouts shared with the TypeScript package's tests.
pub const MESSAGES: &str = include_str!("../../../greybel-io-web/tests/fixtures/messages.json");

pub fn fixture(path: &[&str]) -> Vec<u8> {
    let messages: Value = serde_json::from_str(MESSAGES).expect("valid json");
    let value = path.iter().fold(messages, |value, key| value[key].clone());

    unhex(value.as_str().expect("a fixture in hex"))
}

pub fn keys() -> [(&'static str, Key); 5] {
    [
        ("char", Key::Char('a')),
        ("up", Key::Up),
        ("pageDown", Key::PageDown),
        ("f5", Key::F(5)),
        ("leftShift", Key::Shift(Side::Left)),
    ]
}
