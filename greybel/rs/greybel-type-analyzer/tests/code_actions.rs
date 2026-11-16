//! Tests the quick fixes offered for diagnostics.

use crate::support::code_actions::{fixed, titles};

#[test]
fn should_suggest_close_names_in_scope() {
    let source = "counter = 1\nprint countr";

    let result = fixed(source, 2, "Change to \"counter\"");

    assert_eq!(result.as_deref(), Some("counter = 1\nprint counter"));
}

#[test]
fn should_suggest_close_general_functions() {
    let source = "x = 1\nprnt x";

    let result = fixed(source, 2, "Change to \"print\"");

    assert_eq!(result.as_deref(), Some("x = 1\nprint x"));
}

#[test]
fn should_suggest_statement_keywords_at_the_start_of_a_line() {
    let source = "f = function(x)\n  rerurn x\nend function\nf(1)";

    let result = fixed(source, 2, "Change to \"return\"");

    assert_eq!(
        result.as_deref(),
        Some("f = function(x)\n  return x\nend function\nf(1)")
    );
}

#[test]
fn should_suggest_close_members_of_a_known_map() {
    let source = "m = {\"total\": 1}\nprint m.totl";

    let result = fixed(source, 2, "Change to \"total\"");

    assert_eq!(result.as_deref(), Some("m = {\"total\": 1}\nprint m.total"));
}

#[test]
fn should_suggest_self_for_a_member_of_the_receiver() {
    let source = "Tool = {}\nTool.probe = function\n  return 1\nend function\nTool.run = function\n  return probe\nend function\nTool.run";

    let exact = fixed(source, 6, "Change to \"self.probe\"");
    let close = fixed(
        &source.replace("return probe", "return prbe"),
        6,
        "Change to \"self.probe\"",
    );

    assert_eq!(
        exact.as_deref(),
        Some(source.replace("return probe", "return self.probe").as_str())
    );
    assert_eq!(
        close.as_deref(),
        Some(source.replace("return probe", "return self.probe").as_str())
    );
}

#[test]
fn should_suggest_nothing_for_a_name_far_from_all_others() {
    let source = "alpha = 1\nprint zzzzzz";

    let titles = titles(source, 2);

    assert!(
        titles.iter().all(|title| !title.starts_with("Change to")),
        "{titles:?}"
    );
}

#[test]
fn should_ignore_a_diagnostic_on_its_line() {
    let source = "f = function\n  print missing\nend function";

    let result = fixed(source, 2, "Ignore \"unresolved-reference\" on this line");

    assert_eq!(
        result.as_deref(),
        Some("f = function\n  // @ignore unresolved-reference\n  print missing\nend function")
    );
}

#[test]
fn should_extend_an_ignore_comment_that_names_codes() {
    let source = "// @ignore native-override\nprint = missing";

    let result = fixed(source, 2, "Ignore \"unresolved-reference\" on this line");

    assert_eq!(
        result.as_deref(),
        Some("// @ignore native-override, unresolved-reference\nprint = missing")
    );
}

#[test]
fn should_ignore_a_diagnostic_in_the_file() {
    let fresh = "x = 1\nprint missing";
    let named = "// @ignore-file shadowing\nprint missing";

    let fresh = fixed(fresh, 2, "Ignore \"unresolved-reference\" in this file");
    let named = fixed(named, 2, "Ignore \"unresolved-reference\" in this file");

    assert_eq!(
        fresh.as_deref(),
        Some("// @ignore-file unresolved-reference\nx = 1\nprint missing")
    );
    assert_eq!(
        named.as_deref(),
        Some("// @ignore-file shadowing, unresolved-reference\nprint missing")
    );
}

#[test]
fn should_offer_no_ignore_for_syntax_errors() {
    let source = "x = (1";

    let titles = titles(source, 1);

    assert!(
        titles.iter().all(|title| !title.starts_with("Ignore")),
        "{titles:?}"
    );
}

#[test]
fn should_remove_an_unused_local_assigned_a_value_without_side_effects() {
    let source = "f = function\n  unused = [1, \"a\", {\"k\": -2}]\n  return 1\nend function\nf";

    let result = fixed(source, 2, "Remove the assignment to \"unused\"");

    assert_eq!(
        result.as_deref(),
        Some("f = function\n  return 1\nend function\nf")
    );
}

#[test]
fn should_keep_an_unused_local_whose_value_may_have_side_effects() {
    let call = "f = function\n  unused = launch\n  return 1\nend function\nf";
    let shared_line = "f = function\n  if 1 then unused = 2\n  return 1\nend function\nf";

    let call = titles(call, 2);
    let shared_line = titles(shared_line, 2);

    assert!(
        call.iter().all(|title| !title.starts_with("Remove")),
        "{call:?}"
    );
    assert!(
        shared_line.iter().all(|title| !title.starts_with("Remove")),
        "{shared_line:?}"
    );
}

#[test]
fn should_rename_an_unused_variable_with_an_underscore() {
    let local = "f = function\n  unused = launch\n  unused = 2\n  return 1\nend function\nf";
    let parameter = "f = function(a, b)\n  return a\nend function\nf(1, 2)";

    let local = fixed(local, 2, "Rename to \"_unused\"");
    let parameter = fixed(parameter, 1, "Rename to \"_b\"");

    assert_eq!(
        local.as_deref(),
        Some("f = function\n  _unused = launch\n  _unused = 2\n  return 1\nend function\nf")
    );
    assert_eq!(
        parameter.as_deref(),
        Some("f = function(a, _b)\n  return a\nend function\nf(1, 2)")
    );
}

#[test]
fn should_rename_an_unused_parameter_in_its_doc_comment_too() {
    let source = "// @param {number} b\nf = function(a, b)\n  return a\nend function\nf(1, 2)";

    let result = fixed(source, 2, "Rename to \"_b\"");

    assert_eq!(
        result.as_deref(),
        Some("// @param {number} _b\nf = function(a, _b)\n  return a\nend function\nf(1, 2)")
    );
}

#[test]
fn should_suggest_close_codes_in_ignore_comments() {
    let source = "// @ignore unresolved-refrence, shadowing\nprint missing";

    let result = fixed(source, 1, "Change to \"unresolved-reference\"");

    assert_eq!(
        result.as_deref(),
        Some("// @ignore unresolved-reference, shadowing\nprint missing")
    );
}

#[test]
fn should_remove_unreachable_statements() {
    let source = "f = function(x)\n  return x\n  print 1\n  print 2\nend function\nf(1)";

    let result = fixed(source, 3, "Remove the unreachable statements");

    assert_eq!(
        result.as_deref(),
        Some("f = function(x)\n  return x\nend function\nf(1)")
    );
}

#[test]
fn should_remove_an_unreachable_statement_after_exit() {
    let source = "exit(\"done\")\nprint 1";

    let result = fixed(source, 2, "Remove the unreachable statement");

    assert_eq!(result.as_deref(), Some("exit(\"done\")\n"));
}
