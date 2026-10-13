#[path = "support/programs.rs"]
mod support;

use support::{SESSION, computer, far, remote, terminal_on};

use super::{ProgramTable, TerminalClose, Tree};

#[test]
fn should_put_a_launched_program_in_the_tree_of_its_launcher() {
    // Arrange
    let mut table = ProgramTable::default();
    let root = table
        .add_program(0, Some((computer(), 1)), SESSION, None)
        .expect("an id");
    let child = table
        .add_program(0, Some((computer(), 2)), SESSION, Some(root))
        .expect("an id");

    // Act
    let grandchild = table
        .add_program(0, Some((computer(), 3)), SESSION, Some(child))
        .expect("an id");

    // Assert
    let tree = Tree {
        connection: 0,
        root: root.id,
    };

    assert_eq!(table.tree(0, child), Some(tree));
    assert_eq!(table.tree(0, grandchild), Some(tree));
    assert_eq!(table.depth(grandchild), 2);
}

#[test]
fn should_never_give_an_ended_programs_id_again() {
    // Arrange
    let mut table = ProgramTable::default();
    let first = table
        .add_program(0, Some((computer(), 1)), SESSION, None)
        .expect("an id");

    table.end(0, first);

    // Act
    let second = table
        .add_program(0, Some((computer(), 2)), SESSION, None)
        .expect("an id");

    // Assert
    assert_ne!(first, second);
    assert_eq!(table.session(0, first), None);
}

#[test]
fn should_close_the_terminal_on_a_remote_computer_from_the_player() {
    // Arrange
    let mut table = ProgramTable::default();
    let root = table
        .add_program(0, Some((computer(), 1)), SESSION, None)
        .expect("an id");
    let terminal = terminal_on(&mut table, root, remote(), 1, 0);

    // Act
    let close = table.exit_terminal(0, terminal, computer());

    // Assert
    assert_eq!(
        close,
        Some(TerminalClose {
            source: computer(),
            remote: remote()
        })
    );
    assert_eq!(table.exit_terminal(0, terminal, computer()), None);
}

#[test]
fn should_close_nothing_for_a_terminal_on_the_computer_it_came_from() {
    // Arrange
    let mut table = ProgramTable::default();
    let root = table
        .add_program(0, Some((computer(), 1)), SESSION, None)
        .expect("an id");
    let local = terminal_on(&mut table, root, computer(), 0, 0);
    let bounced = terminal_on(&mut table, root, computer(), 1, 0);

    // Act and assert
    assert_eq!(
        table.exit_terminal(0, bounced, computer()),
        Some(TerminalClose {
            source: computer(),
            remote: computer()
        })
    );
    assert_eq!(table.exit_terminal(0, local, computer()), None);
}

#[test]
fn should_name_the_previous_terminals_computer_as_the_source_of_a_nested_terminal() {
    // Arrange
    let mut table = ProgramTable::default();
    let root = table
        .add_program(0, Some((computer(), 1)), SESSION, None)
        .expect("an id");
    let outer = terminal_on(&mut table, root, remote(), 1, 0);
    let inner = terminal_on(&mut table, outer, far(), 2, 1);

    // Act
    let first = table.exit_terminal(0, inner, computer());
    let second = table.exit_terminal(0, outer, computer());

    // Assert
    assert_eq!(
        first,
        Some(TerminalClose {
            source: remote(),
            remote: far()
        })
    );
    assert_eq!(
        second,
        Some(TerminalClose {
            source: computer(),
            remote: remote()
        })
    );
}
