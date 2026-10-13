//! Tests scope definitions, namespaces, and returns, including rollback of failed statements in unsafe parsing.

use greybel_core::{Parser, parse_unsafe};

use crate::support::greybel_options;
use crate::support::scopes::{function_scopes, names};

#[test]
fn should_record_the_definitions_of_a_function_and_its_outer_scope() {
    let source = "f = function(a)\n  b = 1\n  return b\nend function\nc = 2\n";

    let ast = Parser::new(source, greybel_options()).parse().unwrap();

    let [function] = function_scopes(&ast)[..] else {
        panic!("one function scope");
    };

    assert_eq!(names(&ast, ast.list(function.definitions)), ["a", "b"]);
    assert_eq!(ast.list(function.returns).len(), 1);
    assert_eq!(names(&ast, ast.list(ast.scope().definitions)), ["f", "c"]);
}

#[test]
fn should_keep_a_closed_function_scope_when_its_end_statement_is_undone() {
    let source = "f = function\n  a = 1\nend function )\n  b = 2\nend function\n";

    let output = parse_unsafe(source, greybel_options());

    assert!(output.all_errors().next().is_some());

    let [function] = function_scopes(&output.ast)[..] else {
        panic!("one function scope");
    };

    assert_eq!(
        names(&output.ast, output.ast.list(function.definitions)),
        ["a", "b"]
    );
    assert_eq!(
        names(&output.ast, output.ast.list(output.ast.scope().definitions)),
        ["f"]
    );
}

#[test]
fn should_drop_the_scope_of_a_function_opened_by_an_undone_statement() {
    let source = "f = function )\nx = 1\n";

    let output = parse_unsafe(source, greybel_options());

    assert!(output.all_errors().next().is_some());
    assert!(function_scopes(&output.ast).is_empty());
    assert_eq!(
        names(&output.ast, output.ast.list(output.ast.scope().definitions)),
        ["x"]
    );
}
