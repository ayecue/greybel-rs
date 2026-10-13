//! Random-input tests asserting that no dialect's lexer or parser panics. `FUZZ_INPUTS` increases the input count.

use greybel_core::{Dialect, Version};

use crate::support::fuzz::{NamedParse, input_count, panicking_inputs, parse_all_modes};

#[test]
fn should_not_panic_on_random_input() {
    let inputs = input_count();
    let dialects: [NamedParse; 4] = [
        ("MiniScript latest", |s| {
            parse_all_modes(Dialect::MiniScript, Version::Latest, s)
        }),
        ("MiniScript 1.6", |s| {
            parse_all_modes(Dialect::MiniScript, Version::V1_6, s)
        }),
        ("Greybel", |s| {
            parse_all_modes(Dialect::Greybel, Version::Latest, s)
        }),
        ("GreyScript", |s| {
            parse_all_modes(Dialect::GreyScript, Version::V1_6, s)
        }),
    ];

    let report = panicking_inputs(inputs, &dialects);

    assert!(
        report.is_empty(),
        "inputs that panic:\n{}",
        report.join("\n")
    );
}
