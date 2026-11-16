//! Tests paths ended by `return`, `break` and `continue`, and loop exits.

use crate::support::narrowing::ITEM;
use crate::support::{greyscript, miniscript};

#[test]
fn should_return_null_when_the_end_of_a_function_is_reachable() {
    let source = "f = function(x)\n  if x then return 1\nend function\nr = f(1)";

    let types = miniscript(source, &["r"]);

    assert_eq!(types, ["null or number"]);
}

#[test]
fn should_not_return_null_when_every_path_returns() {
    let branches = "f = function(x)\n  if x then\n    return 1\n  else\n    return 2\n  end if\nend function\nr = f(1)";
    let endless = "f = function(x)\n  while true\n    if x then return 1\n  end while\nend function\nr = f(1)";

    let branches = miniscript(branches, &["r"]);
    let endless = miniscript(endless, &["r"]);

    assert_eq!(branches, ["number"]);
    assert_eq!(endless, ["number"]);
}

#[test]
fn should_narrow_after_a_branch_that_returns() {
    let source = "f = function(item)\n  if item == null then return \"\"\n  return item\nend function\nr = f(\"a\")\ns = f(null)";

    let types = greyscript(source, &["r", "s"]);

    assert_eq!(types, ["string", "string"]);
}

#[test]
fn should_narrow_after_a_branch_that_leaves_an_iteration() {
    let after_break =
        format!("{ITEM}while true\n  if item isa string then break\n  number = item\nend while");
    let after_continue =
        format!("{ITEM}for i in [1]\n  if item isa string then continue\n  number = item\nend for");

    let after_break = miniscript(&after_break, &["number"]);
    let after_continue = miniscript(&after_continue, &["number"]);

    assert_eq!(after_break, ["number"]);
    assert_eq!(after_continue, ["number"]);
}

#[test]
fn should_leave_an_endless_loop_only_at_a_break() {
    let source =
        format!("{ITEM}while true\n  if item isa string then break\nend while\nafter = item");

    let types = miniscript(&source, &["after"]);

    assert_eq!(types, ["string"]);
}

#[test]
fn should_narrow_a_loop_body_and_exit_by_the_condition() {
    let source = "item = null\nitem = \"a\"\nwhile item != null\n  inside = item\n  item = null\nend while\nafter = item";

    let types = miniscript(source, &["inside", "after"]);

    assert_eq!(types, ["string", "null"]);
}

#[test]
fn should_end_the_path_at_a_call_of_exit() {
    let source = "item = null\nitem = \"a\"\nif item == null then exit(\"none\")\nafter = item";

    let types = greyscript(source, &["after"]);

    assert_eq!(types, ["string"]);
}

#[test]
fn should_type_a_function_that_always_exits_as_never() {
    let source = "fail = function(message)\n  exit(message)\nend function\nf = function(x)\n  if x then return 1\n  fail(\"no\")\nend function\nr = f(1)\nfailed = fail(\"x\")";

    let types = greyscript(source, &["r", "failed"]);

    assert_eq!(types, ["number", "never"]);
}

#[test]
fn should_end_the_path_at_a_call_returning_never_by_its_tag() {
    let source = "// @return {never}\nstop = function\nend function\nf = function(x)\n  if x then return 1\n  stop\nend function\nr = f(1)";

    let types = miniscript(source, &["r"]);

    assert_eq!(types, ["number"]);
}
