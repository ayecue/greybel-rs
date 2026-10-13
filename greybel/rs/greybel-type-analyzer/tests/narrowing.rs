//! Tests type narrowing in `if` branches and branch-specific assignments.

use crate::support::narrowing::{ITEM, THREE, branches};
use crate::support::{greyscript, miniscript};

#[test]
fn should_narrow_both_branches_by_isa_and_typeof() {
    let (two, three) = (ITEM, THREE);

    let isa = branches(two, "item isa string");
    let typeof_left = branches(two, "typeof(item) == \"string\"");
    let typeof_right = branches(two, "\"string\" == typeof(item)");
    let typeof_not_equal = branches(two, "typeof(item) != \"string\"");
    let not_isa = branches(two, "not item isa string");
    let isa_map = branches(three, "item isa map");

    assert_eq!(isa, ["string", "number"]);
    assert_eq!(typeof_left, ["string", "number"]);
    assert_eq!(typeof_right, ["string", "number"]);
    assert_eq!(typeof_not_equal, ["number", "string"]);
    assert_eq!(not_isa, ["number", "string"]);
    assert_eq!(isa_map, ["map", "number or string"]);
}

#[test]
fn should_narrow_by_null_checks() {
    let prelude = "item = null\nitem = \"hello\"\n";

    let equal = branches(prelude, "item == null");
    let not_equal = branches(prelude, "null != item");

    assert_eq!(equal, ["null", "string"]);
    assert_eq!(not_equal, ["string", "null"]);
}

#[test]
fn should_combine_conjunctions_and_disjunctions() {
    let contradiction = "item = {}\nitem = \"hello\"\nif item isa map and typeof(item) == \"string\" then\n  output = item\nend if";

    let conjunction = branches(ITEM, "item isa string and typeof(item) == \"string\"");
    let negated = branches(ITEM, "not (item isa string and typeof(item) == \"string\")");
    let disjunction = branches(
        THREE,
        "typeof(item) == \"string\" or typeof(item) == \"number\"",
    );
    let contradiction = miniscript(contradiction, &["output"]);

    assert_eq!(conjunction, ["string", "number"]);
    assert_eq!(negated, ["number", "string"]);
    assert_eq!(disjunction, ["number or string", "map"]);
    assert_eq!(contradiction, ["unknown"]);
}

#[test]
fn should_carry_earlier_negations_through_else_if_chains() {
    let source = format!(
        "{THREE}if item isa string then\n  a = item\nelse if item isa number then\n  b = item\nelse\n  c = item\nend if\nif item isa string then\n  d = item\nelse\n  if item isa number then\n    e = item\n  else\n    f = item\n  end if\nend if"
    );

    let types = miniscript(&source, &["a", "b", "c", "d", "e", "f"]);

    assert_eq!(
        types,
        ["string", "number", "map", "string", "number", "map"]
    );
}

#[test]
fn should_hold_outer_facts_inside() {
    let source = "source = 1
        source = \"oops\"
        label = 1
        label = \"ok\"
        if source isa number then
          if source isa number and label isa string then
            positive = label
          else
            negative = label
          end if
        end if";

    let types = miniscript(source, &["positive", "negative"]);

    assert_eq!(types, ["string", "number"]);
}

#[test]
fn should_end_the_narrowing_on_assignment() {
    let source = format!(
        "{ITEM}if item isa string then\n  before = item\n  item = 123\n  after = item\nend if"
    );

    let types = miniscript(&source, &["before", "after"]);

    assert_eq!(types, ["string", "number or string"]);
}

#[test]
fn should_write_properties_onto_the_narrowed_maps() {
    let source = format!("{THREE}if item isa map then\n  item.foo = 1\nend if\nresult = item.foo");

    let types = miniscript(&source, &["result"]);

    assert_eq!(types, ["number"]);
}

#[test]
fn should_give_unknown_values_the_tested_kind() {
    let source = "item = get_custom_object.value\nif typeof(item) == \"string\" then\n  output = item\nend if";

    let types = greyscript(source, &["output"]);

    assert_eq!(types, ["string"]);
}

#[test]
fn should_keep_the_others_narrowed_by_a_guard_on_another_variable() {
    let prelude = format!("{ITEM}guard = 1\n");

    let not_isa = branches(&prelude, "guard isa number and not item isa string");
    let typeof_not_equal = branches(&prelude, "guard isa number and typeof(item) != \"string\"");

    assert_eq!(not_isa, ["number", "string"]);
    assert_eq!(typeof_not_equal, ["number", "string"]);
}

#[test]
fn should_narrow_by_typeof_what_nothing_assigned() {
    let source = "item = get_custom_object.item
            if typeof(item) == \"string\" then
              output = item
            end if";

    let types = greyscript(source, &["output"]);

    assert_eq!(types, ["string"]);
}

#[test]
fn should_land_entries_written_in_a_branch_on_the_map() {
    let source = "item = {}
            item = \"hello\"
            if typeof(item) == \"string\" then
              positive = item
            else
              item.answer = 123
            end if
            output = item.answer";

    let types = greyscript(source, &["item", "output"]);

    assert_eq!(types, ["map or string", "number"]);
}
