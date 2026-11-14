use greybel_test_support::future::block_on;
use greyhack_behaviour::SAFE_MODE_TERMINAL;

use crate::support::safe_mode::{
    LAUNCH, LAUNCH_PARENT, SETUP, SETUP_OUTPUT, SETUP_PARENT, SETUP_PARENT_OUTPUT, run, run_safe,
    world,
};

/// A normal terminal needs the execute permission of the file and the access of its folders, in that order of
/// refusal (`ShellIntrinsics.launch`).
#[test]
fn should_refuse_a_launch_without_the_execute_permission() {
    let (setup, world) = run(SETUP, world());

    assert_eq!(setup, SETUP_OUTPUT);

    let (output, world) = run(LAUNCH, world);

    assert_eq!(
        output,
        "Can't launch program. Permission denied.\n0\nCan't launch program. Permission denied.\n0"
    );
    assert!(
        !block_on(
            world
                .modules
                .core
                .computers
                .processes
                .in_safe_mode(world.handle.player())
        )
        .unwrap()
    );
}

/// A restart in safe mode starts the protected safe-mode terminal, whose programs launch files without the execute
/// permission but still need the access of their folders (`GreyInterpreter.isSafeTerminal`).
#[test]
fn should_launch_without_the_execute_permission_in_the_safe_mode_terminal() {
    let (setup, world) = run(SETUP, world());

    assert_eq!(setup, SETUP_OUTPUT);

    let (output, world) = run("print get_shell.host_computer.reboot(1)", world);

    assert_eq!(output, "1");
    assert!(
        block_on(
            world
                .modules
                .core
                .computers
                .processes
                .in_safe_mode(world.handle.player())
        )
        .unwrap()
    );

    let (output, _) = run_safe(LAUNCH, world);

    assert_eq!(
        output,
        "ran\n1\nCan't launch program. Permission denied.\n0"
    );
}

/// The flag belongs to the terminal a program is typed into, not to the computer: a program that is not typed into
/// the safe-mode terminal while one runs still needs the execute permission (`GreyInterpreter.isSafeTerminal`).
#[test]
fn should_refuse_a_launch_of_a_program_outside_the_safe_mode_terminal() {
    let (setup, world) = run(SETUP, world());

    assert_eq!(setup, SETUP_OUTPUT);

    let (_, world) = run("print get_shell.host_computer.reboot(1)", world);

    assert!(
        block_on(
            world
                .modules
                .core
                .computers
                .processes
                .in_safe_mode(world.handle.player())
        )
        .unwrap()
    );

    let (output, _) = run(LAUNCH, world);

    assert_eq!(
        output,
        "Can't launch program. Permission denied.\n0\nCan't launch program. Permission denied.\n0"
    );
}

/// A program is in the terminal of the program that launched it (`ShellIntrinsics.launch` continues the launching
/// interpreter), so what a safe-mode terminal program launches launches without the execute permission too.
#[test]
fn should_run_the_programs_a_safe_mode_terminal_program_launches_in_the_safe_mode_terminal() {
    let (setup, world) = run(SETUP, world());

    assert_eq!(setup, SETUP_OUTPUT);

    let (parent, world) = run(SETUP_PARENT, world);

    assert_eq!(parent.trim(), SETUP_PARENT_OUTPUT);

    let (_, world) = run("print get_shell.host_computer.reboot(1)", world);
    let (output, _) = run_safe(LAUNCH_PARENT, world);

    assert_eq!(output, "ran\n1\n1");
}

/// The same program launches nothing without the execute permission when it is not typed into the safe-mode terminal.
#[test]
fn should_not_run_the_programs_another_terminal_program_launches_in_the_safe_mode_terminal() {
    let (setup, world) = run(SETUP, world());

    assert_eq!(setup, SETUP_OUTPUT);

    let (parent, world) = run(SETUP_PARENT, world);

    assert_eq!(parent.trim(), SETUP_PARENT_OUTPUT);

    let (_, world) = run("print get_shell.host_computer.reboot(1)", world);
    let (output, _) = run(LAUNCH_PARENT, world);

    assert_eq!(output, "Can't launch program. Permission denied.\n0\n1");
}

/// The safe-mode terminal is a process of the computer until it restarts normally.
#[test]
fn should_list_the_safe_mode_terminal_until_the_next_normal_restart() {
    let (_, world) = run("print get_shell.host_computer.reboot(1)", world());
    let (output, world) = run("print get_shell.host_computer.show_procs", world);

    assert!(output.contains(SAFE_MODE_TERMINAL), "{output}");

    let (_, world) = run("print get_shell.host_computer.reboot", world);
    let (output, _) = run("print get_shell.host_computer.show_procs", world);

    assert!(!output.contains(SAFE_MODE_TERMINAL), "{output}");
}

/// An ordinary restart never starts it.
#[test]
fn should_not_start_the_safe_mode_terminal_on_an_ordinary_restart() {
    let (_, world) = run("print get_shell.host_computer.reboot", world());

    assert!(
        !block_on(
            world
                .modules
                .core
                .computers
                .processes
                .in_safe_mode(world.handle.player())
        )
        .unwrap()
    );
}
