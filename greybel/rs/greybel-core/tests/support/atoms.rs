use greybel_core::{Atom, AtomTable};

pub fn foo_and_bar() -> (AtomTable, Atom, Atom) {
    let mut atoms = AtomTable::default();
    let foo_atom = atoms.insert("foo");
    let bar_atom = atoms.insert("bar");

    atoms.insert("foo");

    (atoms, foo_atom, bar_atom)
}
