use greybel_transpiler::names::*;

use crate::support::name_table::{created, table_with};

#[test]
fn should_count_names_up_over_the_charset() {
    let mut names = NameTable::new(vec!['A', 'B']);
    let originals = ["a", "b", "c", "d", "e", "f", "g"];
    let given: Vec<_> = originals
        .iter()
        .map(|name| created(&mut names, name))
        .collect();

    assert_eq!(given, ["A", "BA", "AB", "BBA", "AAB", "ABA", "ABB"]);
}

#[test]
fn should_give_a_created_name_again() {
    let mut names = table_with(&['A', 'B'], &[], &["a", "b", "c", "d", "e", "f", "g"]);
    let name = names.create("c");

    assert_eq!(name, Some("AB"));
}

#[test]
fn should_not_name_a_forbidden_original() {
    let mut names = table_with(&['A', 'B', 'C'], &["B"], &[]);
    let name = names.create("B");

    assert_eq!(name, None);
}

#[test]
fn should_skip_forbidden_names() {
    let mut names = table_with(&['A', 'B', 'C'], &["B"], &["B"]);
    let name = names.create("x");

    assert_eq!(name, Some("A"));
}

#[test]
fn should_skip_a_name_equal_to_the_original() {
    let mut names = table_with(&['A', 'B', 'C'], &["B"], &["B", "x"]);
    let name = names.create("C");

    assert_eq!(name, Some("CA"));
}

#[test]
fn should_not_get_a_name_for_a_forbidden_original() {
    let names = table_with(&['A', 'B', 'C'], &["B"], &["B", "x", "C"]);
    let name = names.get("B");

    assert_eq!(name, None);
}

#[test]
fn should_give_the_first_name_to_the_first_original() {
    let mut names = table_with(&['A', 'B', 'C', 'D'], &[], &[]);
    let name = names.create("foo");

    assert_eq!(name, Some("A"));
}

#[test]
fn should_name_an_original_equal_to_a_given_name() {
    let mut names = table_with(&['A', 'B', 'C', 'D'], &[], &["foo"]);
    let name = names.create("A");

    assert_eq!(name, Some("B"));
}

#[test]
fn should_rename_the_earlier_original_on_a_clash() {
    let names = table_with(&['A', 'B', 'C', 'D'], &[], &["foo", "A"]);
    let name = names.get("foo");

    assert_eq!(name, Some("C"));
}
