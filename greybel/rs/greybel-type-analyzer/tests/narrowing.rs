//! Tests type narrowing in `if` branches and branch-specific assignments.

use greybel_type_analyzer::{Analyzer, Dialect};

use crate::support::narrowing::{ITEM, NULLABLE, SHELL, THREE, branches, function_locals};
use crate::support::{PATH, greyscript, locals, marked, miniscript, uri, with_document};

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
fn should_narrow_by_isa_against_a_map_of_the_script() {
    let source = "A = {\"a\": 1}
        B = {\"b\": \"x\"}
        item = new A
        if rnd then item = new B
        if rnd then item = 1
        if item isa A then
          positive = item.a
        else
          negative = item
        end if
        other = {}
        if other isa A then instance = other.a";

    let types = miniscript(source, &["positive", "negative", "instance"]);

    assert_eq!(types, ["number", "map or number", "number"]);
}

#[test]
fn should_narrow_by_null_checks() {
    let prelude = "item = null\nif rnd then item = \"hello\"\n";

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
        if rnd then source = \"oops\"
        label = 1
        if rnd then label = \"ok\"
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

    assert_eq!(types, ["string", "number"]);
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
            if rnd then item = \"hello\"
            if typeof(item) == \"string\" then
              positive = item
            else
              item.answer = 123
            end if
            output = item.answer";

    let types = greyscript(source, &["item", "output"]);

    assert_eq!(types, ["map or string", "number"]);
}

#[test]
fn should_narrow_else_if_conditions_by_the_earlier_conditions() {
    let (source, positions) = marked(&format!(
        "{THREE}if item isa map then\n  print 1\nelse if |item isa string then\n  print 2\nend if"
    ));
    let workspace = with_document(Dialect::MiniScript, &source);

    let hover = Analyzer::new(&workspace)
        .hover(&uri(PATH), positions[0])
        .map(|hover| hover.text);

    assert_eq!(hover.as_deref(), Some("item: number or string"));
}

#[test]
fn should_not_narrow_outer_variables_in_a_function_defined_in_a_branch() {
    let source = format!(
        "{ITEM}if item isa string then\n  read = function\n    inner = item\n  end function\nend if"
    );
    let workspace = with_document(Dialect::MiniScript, &source);
    let analysis = Analyzer::new(&workspace)
        .analysis(&uri(PATH))
        .expect("the document is loaded");

    let types = locals(analysis, 0, &["inner"]);

    assert_eq!(types, ["number or string"]);
}

#[test]
fn should_narrow_by_truthiness() {
    let truthy = branches(NULLABLE, "item");
    let falsy = branches(NULLABLE, "not item");
    let exits = greyscript(
        &format!("{NULLABLE}if not item then exit(\"none\")\noutput = item"),
        &["output"],
    );

    assert_eq!(truthy, ["string", "null or string"]);
    assert_eq!(falsy, ["null or string", "string"]);
    assert_eq!(exits, ["string"]);
}

#[test]
fn should_narrow_the_right_side_of_and_and_or() {
    let hover = |condition: &str| {
        let (source, positions) = marked(&format!("{NULLABLE}x = {condition}"));
        let workspace = with_document(Dialect::MiniScript, &source);

        Analyzer::new(&workspace)
            .hover(&uri(PATH), positions[0])
            .map(|hover| hover.text)
    };

    assert_eq!(
        hover("item != null and |item.len").as_deref(),
        Some("item: string")
    );
    assert_eq!(
        hover("item == null or |item.len").as_deref(),
        Some("item: string")
    );
    assert_eq!(hover("item and |item.len").as_deref(), Some("item: string"));
}

#[test]
fn should_narrow_members_by_null_checks_truthiness_and_isa() {
    let source = format!(
        "{SHELL}if m.shell != null then positive = m.shell
        if m.shell then truthy = m.shell
        if m[\"shell\"] isa string then indexed = m.shell
        if not m.shell then exit(\"none\")
        after = m.shell"
    );

    let types = greyscript(&source, &["positive", "truthy", "indexed", "after"]);

    assert_eq!(types, ["string", "string", "string", "string"]);
}

#[test]
fn should_narrow_members_of_self() {
    let source = "Session = {\"shell\": null}
        Session.connect = function
          self.shell = \"x\"
        end function
        Session.run = function
          if self.shell == null then return
          output = self.shell
        end function";

    let types = function_locals(source, 1, &["output"]);

    assert_eq!(types, ["string"]);
}

#[test]
fn should_read_a_member_as_its_last_assignment() {
    let source = "m = {}
        m.v = \"a\"
        if rnd then m.v = 1
        m.v = [1]
        assigned = m.v
        m[\"v\"] = \"b\"
        indexed = m.v";

    let types = greyscript(source, &["assigned", "indexed"]);

    assert_eq!(types, ["list<number>", "string"]);
}

#[test]
fn should_drop_member_facts_at_assignments_and_calls_that_change_the_member() {
    let alias = greyscript(
        &format!(
            "{SHELL}if m.shell != null then\n  other = m\n  other.shell = null\n  after = m.shell\nend if"
        ),
        &["after"],
    );
    let base = greyscript(
        &format!("{SHELL}n = m\nif m.shell != null then\n  m = n\n  after = m.shell\nend if"),
        &["after"],
    );
    let function = greyscript(
        &format!(
            "{SHELL}reset = function\n  m.shell = null\nend function\nif m.shell != null then\n  reset\n  after = m.shell\nend if"
        ),
        &["after"],
    );
    let computed = greyscript(
        &format!(
            "{SHELL}k = \"shell\"\nif m.shell != null then\n  m[k] = null\n  after = m.shell\nend if"
        ),
        &["after"],
    );
    let unknown = function_locals(
        &format!(
            "{SHELL}run = function(callback)\n  if m.shell != null then\n    callback(1)\n    after = m.shell\n  end if\nend function"
        ),
        0,
        &["after"],
    );
    let list = greyscript(
        &format!("{SHELL}l = [1]\nif m.shell != null then\n  l[0] = 2\n  after = m.shell\nend if"),
        &["after"],
    );

    assert_eq!(alias, ["null or string"]);
    assert_eq!(base, ["null or string"]);
    assert_eq!(function, ["null or string"]);
    assert_eq!(computed, ["null or string"]);
    assert_eq!(unknown, ["null or string"]);
    assert_eq!(list, ["string"]);
}

#[test]
fn should_widen_a_member_fact_by_mutations() {
    let member = greyscript(
        "m = {}\nm.items = []\nm.items.push(1)\noutput = m.items",
        &["output"],
    );
    let variable = greyscript("items = []\nitems.push(1)\noutput = items", &["output"]);

    assert_eq!(member, variable);
}
