//! Tests inlay hints.

use greybel_type_analyzer::InlayHintKind::{Parameter, Type};

use crate::support::inlay::{hints, only};

#[test]
fn should_show_the_type_of_a_variable_at_its_first_assignment() {
    let source = "x = [1, 2]\nx = [3]\ny = 1\nz = x.len";

    assert_eq!(
        hints(source, only(true, false, false)),
        [
            (1, 2, ": list<number>".to_owned(), Type),
            (4, 2, ": number".to_owned(), Type),
        ]
    );
}

#[test]
fn should_name_the_parameters_of_literal_arguments() {
    let source =
        "f = function(host, port)\nend function\nf(\"1.1.1.1\", 22)\nf(h, -1)\nf(1)\nprint \"x\"";

    assert_eq!(
        hints(source, only(false, true, false)),
        [
            (3, 3, "host:".to_owned(), Parameter),
            (3, 14, "port:".to_owned(), Parameter),
            (4, 6, "port:".to_owned(), Parameter),
        ]
    );
}

#[test]
fn should_show_return_types_of_functions_without_a_return_tag() {
    let source = "f = function(a = \"(\")\n  return 1\nend function\n// @return {string}\ng = function\n  return 1\nend function\nh = function\nend function";

    assert_eq!(
        hints(source, only(false, false, true)),
        [
            (1, 22, ": number".to_owned(), Type),
            (8, 13, ": null".to_owned(), Type),
        ]
    );
}
