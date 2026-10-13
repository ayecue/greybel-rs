#[path = "support/mod.rs"]
mod support;

use support::interpolated;

#[test]
fn should_put_the_values_of_expressions_into_a_log_message() {
    let text = interpolated("x is {x}, name is {name}, {{braces}} and {missing}");

    assert_eq!(
        text,
        "x is 42, name is n\"eo, {braces} and {error: Undefined Identifier: 'missing'}"
    );
}
