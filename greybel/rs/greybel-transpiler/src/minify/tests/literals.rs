#[path = "support/literals.rs"]
mod support;

use support::{number, string};

use super::*;
use crate::emit::LineBreakStyle;

#[test]
fn should_hoist_after_three_long_enough_occurrences() {
    let mut variables = NameTable::new(vec!['A', 'B']);
    let mut literals = LiteralTable::default();

    for _ in 0..3 {
        literals.add(&string(), "\"abcd\"", &mut variables);
    }

    let hoisted = literals.hoisted_variable(&string(), "\"abcd\"");

    assert_eq!(hoisted, Some("A"));
}

#[test]
fn should_not_hoist_literals_too_short_to_pay_off() {
    let mut variables = NameTable::new(vec!['A', 'B']);
    let mut literals = LiteralTable::default();

    for _ in 0..3 {
        literals.add(&string(), "\"abcd\"", &mut variables);
    }

    for _ in 0..3 {
        literals.add(&number(), "12", &mut variables);
    }

    let hoisted = literals.hoisted_variable(&number(), "12");

    assert_eq!(hoisted, None);
}

#[test]
fn should_include_the_negation_in_the_key() {
    let negated = Literal {
        value: LiteralValue::Number(1.5),
        negated: true,
    };
    let key = key(&negated, "1.5");

    assert_eq!(key, "-1.5");
}

#[test]
fn should_borrow_the_key_of_a_literal_that_is_not_negated() {
    let literal = string();
    let key = key(&literal, "\"abcd\"");

    assert!(matches!(key, Cow::Borrowed(_)));
}

#[test]
fn should_list_hoisted_multiline_strings_after_the_other_literals() {
    let mut variables = NameTable::new(vec!['A', 'B']);
    let mut literals = LiteralTable::default();

    for _ in 0..3 {
        literals.add(&string(), "\"ab\ncd\"", &mut variables);
    }

    for _ in 0..3 {
        literals.add(&string(), "\"abcdef\"", &mut variables);
    }

    let entries: Vec<_> = literals
        .hoisted_entries(LineBreakStyle::Join {
            function: "m",
            compact: true,
        })
        .collect();

    assert_eq!(
        entries,
        [
            ("BA", "\"abcdef\"".to_owned()),
            ("A", "m([\"ab\",\"cd\"])".to_owned())
        ]
    );
}
