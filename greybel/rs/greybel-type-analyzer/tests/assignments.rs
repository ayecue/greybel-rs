//! Tests the types of variables where they are read: the last assignment on each path, joined where paths meet.

use greybel_type_analyzer::{Analyzer, DiagnosticKind, Dialect};

use crate::support::assignments::NUMBER_FACT;
use crate::support::diagnostics::{messages_of_kind, owned};
use crate::support::{PATH, greyscript, marked, miniscript, uri, with_document};

#[test]
fn should_read_the_last_assignment() {
    let source = "port = params[0]\nfirst = port\nport = port.to_int\nsecond = port";

    let types = greyscript(source, &["first", "second", "port"]);

    assert_eq!(
        types,
        [
            "string",
            "null or number or string",
            "null or number or string"
        ]
    );
}

#[test]
fn should_join_the_assignments_of_branches() {
    let source = "x = 1\nif rnd then\n  x = \"a\"\nelse if rnd then\n  x = []\nend if\njoined = x\nx = {}\nlast = x";

    let types = miniscript(source, &["joined", "last", "x"]);

    assert_eq!(
        types,
        [
            "list<unknown> or number or string",
            "map",
            "list<unknown> or map or number or string"
        ]
    );
}

#[test]
fn should_check_arguments_by_the_value_at_the_call() {
    let source = "// @param {number} n\ndouble = function(n)\n  return n * 2\nend function\nx = \"5\"\na = double(x)\nx = x.to_int\nb = double(x)";
    let workspace = with_document(Dialect::GreyScript, source);

    let reported = messages_of_kind(&workspace, DiagnosticKind::CallSignatureMismatch);

    assert_eq!(
        reported,
        owned(&[(6, "Argument 1 expects number but got string.")])
    );
}

#[test]
fn should_see_assignments_of_earlier_iterations_in_a_loop() {
    let source =
        "text = \"\"\nfor i in range(3)\n  read = text\n  text = text.len\nend for\nafter = text";

    let types = miniscript(source, &["read", "after"]);

    assert_eq!(types, ["number or string", "number or string"]);
}

#[test]
fn should_see_assignments_of_earlier_iterations_in_nested_loops() {
    let source =
        "a = 0\nfor i in [1]\n  for j in [1]\n    read = a\n    a = \"s\"\n  end for\nend for";

    let types = miniscript(source, &["read"]);

    assert_eq!(types, ["number or string"]);
}

#[test]
fn should_see_assignments_passed_along_through_several_iterations() {
    let source = "a = 0\nb = 0\nc = 0\nfor i in range(3)\n  read = a\n  a = b\n  b = c\n  c = \"s\"\nend for";

    let types = miniscript(source, &["read"]);

    assert_eq!(types, ["number or string"]);
}

#[test]
fn should_leave_an_endless_loop_with_the_state_at_its_break() {
    let source = "x = null\nwhile true\n  if x != null then break\n  x = 1\nend while\nafter = x";

    let types = miniscript(source, &["after"]);

    assert_eq!(types, ["number"]);
}

#[test]
fn should_read_what_a_mutation_added() {
    let source = "items = []\nitems.push(1)\nfirst = items[0]";

    let types = miniscript(source, &["first"]);

    assert_eq!(types, ["number"]);
}

#[test]
fn should_read_a_reused_variable_by_its_last_assignment_after_an_exit() {
    let source = "result = get_shell.host_computer.File(\"/etc/passwd\")\nif result == null then exit(\"no file\")\nfile = result\nresult = result.get_content\ncontent = result";

    let types = greyscript(source, &["file", "content"]);

    assert_eq!(types, ["file or string", "null or string"]);
}

#[test]
fn should_drop_the_facts_of_variables_a_call_assigns() {
    let source = "result = null\nrun = function\n  globals.result = \"x\"\nend function\nbefore = result\nrun\nafter = result";

    let types = miniscript(source, &["before", "after"]);

    assert_eq!(types, ["null", "null or string"]);
}

#[test]
fn should_drop_the_facts_of_variables_assigned_by_functions_a_call_reaches() {
    let source = format!(
        "{NUMBER_FACT}inner = function\n  globals.x = \"s\"\nend function\nrun = function\n  inner\nend function\nrun\nafter = x"
    );

    let types = miniscript(&source, &["after"]);

    assert_eq!(types, ["number or string"]);
}

#[test]
fn should_drop_every_fact_at_opaque_calls() {
    let callback = format!(
        "{NUMBER_FACT}// @param {{function}} callback\nrun = function(callback)\n  callback\nend function\nf = function\nend function\nrun(@f)\nafter = x"
    );
    let computed = format!(
        "{NUMBER_FACT}set = function(name)\n  globals[name] = \"s\"\nend function\nset(\"x\")\nafter = x"
    );

    let callback = miniscript(&callback, &["after"]);
    let computed = miniscript(&computed, &["after"]);

    assert_eq!(callback, ["number or string"]);
    assert_eq!(computed, ["number or string"]);
}

#[test]
fn should_keep_the_facts_of_own_variables_at_opaque_calls() {
    let body = "  y = 1\n  if rnd then y = \"s\"\n  y = 1\n  callback\n  return y";
    let own = format!(
        "// @param {{function}} callback\nrun = function(callback)\n{body}\nend function\nf = function\nend function\nr = run(@f)"
    );
    let shared = format!(
        "// @param {{function}} callback\nrun = function(callback)\n  share = @locals\n{body}\nend function\nf = function\nend function\nr = run(@f)"
    );

    let own = miniscript(&own, &["r"]);
    let shared = miniscript(&shared, &["r"]);

    assert_eq!(own, ["number"]);
    assert_eq!(shared, ["number or string"]);
}

#[test]
fn should_keep_facts_across_calls_that_assign_no_outer_variable() {
    let source = format!(
        "{NUMBER_FACT}run = function\n  x = \"local\"\n  locals.y = 1\nend function\nrun\nkept = x"
    );

    let types = miniscript(&source, &["kept"]);

    assert_eq!(types, ["number"]);
}

#[test]
fn should_hover_an_assignment_with_the_assigned_and_the_declared_type() {
    let (source, positions) = marked("|x = 1\nif rnd then |x = \"s\"\nread = |x\n|single = 1");
    let workspace = with_document(Dialect::MiniScript, &source);
    let hover = |position| {
        Analyzer::new(&workspace)
            .hover(&uri(PATH), position)
            .map(|hover| hover.text)
    };

    let texts: Vec<Option<String>> = positions.into_iter().map(hover).collect();

    assert_eq!(
        texts,
        [
            Some("x: number (declared: number or string)".to_owned()),
            Some("x: string (declared: number or string)".to_owned()),
            Some("x: number or string".to_owned()),
            Some("single: number".to_owned()),
        ]
    );
}

#[test]
fn should_read_the_outer_variable_before_a_locals_first_assignment() {
    let source = "x = \"a\"\nf = function\n  y = x\n  x = 1\n  return y\nend function\nr = f";

    let types = miniscript(source, &["r"]);

    assert_eq!(types, ["string"]);
}

#[test]
fn should_report_a_local_read_before_its_first_assignment() {
    let source = "f = function\n  print z\n  z = 1\nend function";
    let workspace = with_document(Dialect::MiniScript, source);

    let reported = messages_of_kind(&workspace, DiagnosticKind::UseBeforeDefinition);

    assert_eq!(
        reported,
        owned(&[(2, "\"z\" is used before it is assigned.")])
    );
}

#[test]
fn should_report_self_assignment_only_where_the_variable_is_assigned() {
    let source = "if rnd then\n  x = 1\nelse\n  x = x\nend if\nx = x";
    let workspace = with_document(Dialect::MiniScript, source);

    let reported = messages_of_kind(&workspace, DiagnosticKind::SelfAssignment);

    assert_eq!(reported, owned(&[(6, "\"x\" is assigned to itself.")]));
}
