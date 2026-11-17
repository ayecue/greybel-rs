//! Tests inlay hints.

use greybel_core::{Position, Range};
use greybel_type_analyzer::InlayHintKind::{Parameter, Type};
use greybel_type_analyzer::{Analyzer, Dialect, InlayHintOptions};

use crate::support::documents::load;
use crate::support::inlay::{HintOption, hints, only};
use crate::support::uri;

#[test]
fn should_show_the_type_of_a_variable_at_its_first_assignment() {
    let source = "x = [1, 2]\nx = [3]\ny = 1\nz = x.len";

    assert_eq!(
        hints(source, only(HintOption::VariableTypes)),
        [
            (1, 2, ": list<number>".to_owned(), Type),
            (4, 2, ": number".to_owned(), Type),
        ]
    );
}

#[test]
fn should_name_the_parameter_of_each_argument() {
    let source = "f = function(host, port)\nend function\nf(\"1.1.1.1\", 22)\nf(h, -1)\nf(1)\nprint \"x\"\nf(g(1), m.k)";

    assert_eq!(
        hints(source, only(HintOption::ParameterNames)),
        [
            (3, 3, "host:".to_owned(), Parameter),
            (3, 14, "port:".to_owned(), Parameter),
            (4, 3, "host:".to_owned(), Parameter),
            (4, 6, "port:".to_owned(), Parameter),
            (5, 3, "host:".to_owned(), Parameter),
            (6, 7, "s:".to_owned(), Parameter),
            (7, 3, "host:".to_owned(), Parameter),
            (7, 9, "port:".to_owned(), Parameter),
        ]
    );
}

#[test]
fn should_name_api_parameters() {
    let source = "l = []\nl.insert(0, command)\nset_content(f, join(g, char(10)))";

    assert_eq!(
        hints(source, only(HintOption::ParameterNames)),
        [
            (2, 10, "index:".to_owned(), Parameter),
            (2, 13, "value:".to_owned(), Parameter),
            (3, 13, "self:".to_owned(), Parameter),
            (3, 16, "content:".to_owned(), Parameter),
            (3, 21, "self:".to_owned(), Parameter),
            (3, 24, "delimiter:".to_owned(), Parameter),
            (3, 29, "codePoint:".to_owned(), Parameter),
        ]
    );
}

#[test]
fn should_leave_out_names_of_arguments_with_the_parameter_name() {
    let source = "f = function(host, port)\nend function\nf(HOST, self.port)\nf(host, p)";

    assert_eq!(
        hints(source, only(HintOption::ParameterNames)),
        [(4, 9, "port:".to_owned(), Parameter)]
    );
}

#[test]
fn should_show_the_types_of_parameters_without_a_written_type() {
    let source = "f = function(argc, count = 1)\nend function\n// @param {string} name\ng = function(name, other)\nend function\nh = (x) => x";

    assert_eq!(
        hints(source, only(HintOption::ParameterTypes)),
        [
            (1, 18, ": any".to_owned(), Type),
            (1, 25, ": number".to_owned(), Type),
            (4, 25, ": any".to_owned(), Type),
            (6, 7, ": any".to_owned(), Type),
        ]
    );
}

#[test]
fn should_show_return_types_of_functions_without_a_return_tag() {
    let source = "f = function(a = \"(\")\n  return 1\nend function\n// @return {string}\ng = function\n  return 1\nend function\nh = function\nend function";

    assert_eq!(
        hints(source, only(HintOption::ReturnTypes)),
        [
            (1, 22, ": number".to_owned(), Type),
            (8, 13, ": null".to_owned(), Type),
        ]
    );
}

#[test]
fn should_show_return_types_of_short_functions_methods_and_generic_functions() {
    let source = "f = (x) => 1\nclass A\n\tname(x)\n\t\treturn \"a\"\n\tend function\nend class\nh = function<T>\nend function\nk = function(a = \")\") // (\n\treturn 1\nend function";

    assert_eq!(
        hints(source, only(HintOption::ReturnTypes)),
        [
            (1, 8, ": number".to_owned(), Type),
            (3, 9, ": string".to_owned(), Type),
            (7, 16, ": null".to_owned(), Type),
            (9, 22, ": number".to_owned(), Type),
        ]
    );
}

#[test]
fn should_leave_out_the_type_of_variables_with_a_written_type() {
    let source = "let x: number = [1].len\nlet y = [1].len";

    assert_eq!(
        hints(source, only(HintOption::VariableTypes)),
        [(2, 6, ": number".to_owned(), Type)]
    );
}

#[test]
fn should_name_parameters_of_functions_its_entry_defines() {
    let workspace = load(
        Dialect::Greybel,
        &[
            (
                "/main.src",
                "App = {}\nApp.info = function(txt)\n\treturn txt\nend function\n#include \"lib.src\"",
            ),
            ("/lib.src", "App.info(\"a\")"),
        ],
    );
    let range = Range::new(Position::new(1, 1), Position::new(2, 1));
    let hints: Vec<String> = Analyzer::new(&workspace)
        .inlay_hints(&uri("/lib.src"), range, InlayHintOptions::default())
        .into_iter()
        .map(|hint| hint.label)
        .collect();

    assert_eq!(hints, ["txt:"]);
}
