use greyhack_world_programs::{Stage, TerminalClose};
use support::{id, push, terminal, tree};

use crate::support;

#[test]
fn should_put_launched_programs_on_their_launchers_tree() {
    let mut tree = tree();
    let child = push(&mut tree, 1, 0, 1);
    let grandchild = push(&mut tree, 2, 1, 1);

    assert_eq!(tree.root_id().id, 0);
    assert_eq!(tree.depth(grandchild), 2);
    assert_eq!(
        tree.program(child).and_then(|program| program.parent),
        Some(id(0))
    );
    assert_eq!(tree.current().map(|program| program.id), Some(grandchild));
}

#[test]
fn should_share_safe_terminal_state_with_launched_programs() {
    let mut tree = tree();
    let child = push(&mut tree, 1, 0, 1);

    tree.set_safe_terminal();

    assert!(tree.safe_terminal());
    assert_eq!(tree.active(child).map(|program| program.depth), Some(1));
}

#[test]
fn should_turn_a_popped_program_and_its_children_into_tombstones() {
    let mut tree = tree();
    let child = push(&mut tree, 1, 0, 1);
    let grandchild = push(&mut tree, 2, 1, 1);

    let end = tree.pop(id(0)).expect("the root ends");

    assert_eq!(end.process, Some((1, 1)));
    assert_eq!(end.children.len(), 2);
    assert_eq!(
        tree.program(child).map(|program| program.stage),
        Some(Stage::Termination)
    );
    assert_eq!(
        tree.program(grandchild).map(|program| program.stage),
        Some(Stage::Termination)
    );
    assert!(tree.active(id(0)).is_none());
    assert!(tree.pop(id(0)).is_none());
}

#[test]
fn should_close_nested_terminals_from_their_previous_computer() {
    let mut tree = tree();
    let outer = terminal(&mut tree, 1, 0, 2, 1);
    let inner = terminal(&mut tree, 2, 1, 3, 2);

    assert_eq!(
        tree.exit_terminal(inner, 1),
        Some(TerminalClose {
            source: 2,
            remote: 3,
        })
    );
    assert_eq!(
        tree.exit_terminal(outer, 1),
        Some(TerminalClose {
            source: 1,
            remote: 2,
        })
    );
}

#[test]
fn should_remove_the_last_shell_when_a_terminal_exits() {
    let mut tree = tree();
    let outer = terminal(&mut tree, 1, 0, 2, 1);
    let inner = terminal(&mut tree, 2, 1, 3, 2);

    tree.terminals.push(2);
    tree.terminals.push(3);

    tree.exit_terminal(inner, 1);

    assert_eq!(tree.terminals.as_slice(), &[2]);
    assert_eq!(tree.last_shell(1), 2);

    tree.exit_terminal(outer, 1);

    assert!(tree.terminals.as_slice().is_empty());
    assert_eq!(tree.last_shell(1), 1);
}

#[test]
fn should_find_the_innermost_terminal_and_its_command_process() {
    let mut tree = tree();
    let outer = terminal(&mut tree, 1, 0, 2, 1);
    let command = push(&mut tree, 2, 1, 2);
    let inner = terminal(&mut tree, 3, 2, 3, 2);
    let inner_command = push(&mut tree, 4, 3, 3);

    assert_eq!(tree.terminal_of(id(0)), None);
    assert_eq!(tree.terminal_of(command), Some(outer));
    assert_eq!(tree.terminal_of(inner), Some(inner));
    assert_eq!(tree.terminal_of(inner_command), Some(inner));
    assert_eq!(tree.launcher_of(inner), Some(command));
    assert_eq!(tree.command_process(command), Some((2, 2)));
    assert_eq!(tree.command_process(inner_command), Some((3, 4)));
}

#[test]
fn should_create_one_default_state_value_per_type() {
    #[derive(Debug, Default, PartialEq)]
    struct State(u32);

    let mut tree = tree();

    tree.state_mut::<State>().0 = 42;

    assert_eq!(tree.state::<State>(), &State(42));
}
