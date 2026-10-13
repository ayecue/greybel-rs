use greybel_core::ast::*;

use crate::support::atoms::foo_and_bar;

#[test]
fn should_return_the_same_atom_for_equal_text() {
    let (mut atoms, foo, _) = foo_and_bar();

    let again = atoms.insert("foo");

    assert_eq!(again, foo);
}

#[test]
fn should_return_another_atom_for_other_text() {
    let mut atoms = AtomTable::default();
    let foo = atoms.insert("foo");

    let bar = atoms.insert("bar");

    assert_ne!(foo, bar);
}

#[test]
fn should_resolve_an_atom_to_its_text() {
    let (atoms, foo, bar) = foo_and_bar();

    let [foo_text, bar_text] = [foo, bar].map(|atom| atoms.resolve(atom));

    assert_eq!(foo_text, "foo");
    assert_eq!(bar_text, "bar");
}

#[test]
fn should_get_the_atom_of_inserted_text_only() {
    let (atoms, _, bar) = foo_and_bar();

    let [found, missing] = ["bar", "baz"].map(|text| atoms.get(text));

    assert_eq!(found, Some(bar));
    assert_eq!(missing, None);
}

#[test]
fn should_count_each_text_once() {
    let (atoms, _, _) = foo_and_bar();

    let len = atoms.len();

    assert_eq!(len, 2);
}
