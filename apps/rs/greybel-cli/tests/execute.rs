//! Tests `greybel execute` in simulated worlds with terminal debugging.

use crate::support::{Project, world};

#[test]
fn should_run_a_program_with_its_params_imports_and_variables() {
    let project = Project::new(&[
        (
            "main.src",
            "import_code(\"lib.src\")\nprint(\"<b>\" + double(params[0].to_int) + \"</b> \" + #envar NAME)",
        ),
        (
            "lib.src",
            "double = function(x)\n  return x * 2\nend function",
        ),
    ]);

    let run = project.run(&["execute", "main.src", "-p", "21", "-vr", "NAME=Grey"], "");

    assert!(run.success(), "{run:?}");
    assert_eq!(run.stdout(), "42 Grey\n");
}

#[test]
fn should_run_the_first_entry_of_greybel_json_with_its_settings() {
    let project = Project::new(&[
        (
            "main.src",
            "print(params.join(\",\") + \" \" + #envar NAME)",
        ),
        ("run.env", "NAME=Grey"),
        (
            "greybel.json",
            r#"{
                "entries": ["main.src"],
                "run": { "params": ["a", "b"], "variables": { "files": ["run.env"] } }
            }"#,
        ),
    ]);

    let run = project.run(&["execute"], "");

    assert!(run.success(), "{run:?}");
    assert_eq!(run.stdout(), "a,b Grey\n");
}

#[test]
fn should_run_as_the_user_of_greybel_json_or_the_option() {
    let project = Project::new(&[
        ("main.src", "print(active_user + \" \" + home_dir)"),
        (
            "greybel.json",
            r#"{ "entries": ["main.src"], "run": { "user": "Root" } }"#,
        ),
    ]);

    let root = project.run(&["execute"], "");
    let user = project.run(&["execute", "--user", "user"], "");

    assert!(root.success(), "{root:?}");
    assert_eq!(root.stdout(), "root /root\n");
    assert_eq!(user.stdout(), "test /home/test\n");
}

#[test]
fn should_read_input_from_the_terminal() {
    let project = Project::new(&[(
        "main.src",
        "name = user_input(\"Name: \")\nprint(\"hi \" + name)",
    )]);

    let run = project.run(&["execute", "main.src"], "Ada\n");

    assert!(run.success(), "{run:?}");
    assert!(run.stdout().ends_with("hi Ada\n"), "{run:?}");
}

#[test]
fn should_report_a_runtime_error_with_its_stack() {
    let project = Project::new(&[("main.src", "f = function()\n  return {}.x\nend function\nf")]);

    let run = project.run(&["execute", "main.src"], "");

    assert!(!run.success(), "{run:?}");
    assert!(run.stderr().starts_with("Runtime error: "), "{run:?}");
    assert!(run.stderr().contains("at main.src:2:"), "{run:?}");
}

#[test]
fn should_debug_at_debugger_statements_and_breakpoints() {
    let project = Project::new(&[(
        "main.src",
        "x = 1\ndebugger\ny = x + 1\nprint(y)\nprint(\"end\")",
    )]);

    let run = project.run(
        &["execute", "main.src", "-d", "--break", "main.src:4"],
        "x * 10\nvars\ncontinue\ny = 5\nc\n",
    );
    let stdout = run.stdout();

    assert!(run.success(), "{run:?}");
    assert!(
        stdout.contains("Stopped (debugger statement) at main.src:2:1."),
        "{run:?}"
    );
    assert!(stdout.contains("] > 10\n"), "{run:?}");
    assert!(stdout.contains("  x = 1\n"), "{run:?}");
    assert!(
        stdout.contains("Stopped (breakpoint) at main.src:4:1."),
        "{run:?}"
    );
    assert!(stdout.ends_with("5\nend\n"), "{run:?}");
}

/// Verifies that programs in separate `greybel` processes share state through the connected game server.
#[test]
fn should_run_programs_of_separate_processes_in_the_world_of_a_game_server() {
    let address = world::serve();
    let project = Project::new(&[
        (
            "write.src",
            "computer = get_shell.host_computer\ncomputer.touch(home_dir, \"shared.txt\")\ncomputer.File(home_dir + \"/shared.txt\").set_content(\"from the writer\")",
        ),
        (
            "read.src",
            "print(get_shell.host_computer.File(home_dir + \"/shared.txt\").get_content)",
        ),
    ]);

    let write = project.run(&["execute", "write.src", "--world", &address], "");
    let read = project.run(&["execute", "read.src", "--world", &address], "");

    assert!(write.success(), "{write:?}");
    assert!(read.success(), "{read:?}");
    assert_eq!(read.stdout(), "from the writer\n");
}

/// Verifies that programs in separate `greybel` processes share the world server the first one starts.
#[test]
fn should_run_programs_of_separate_processes_in_the_shared_world() {
    let port = std::net::TcpListener::bind(("127.0.0.1", 0))
        .and_then(|listener| listener.local_addr())
        .expect("a free port is found")
        .port()
        .to_string();
    let project = Project::new(&[
        (
            "write.src",
            "computer = get_shell.host_computer\ncomputer.touch(home_dir, \"shared.txt\")\ncomputer.File(home_dir + \"/shared.txt\").set_content(\"from the writer\")",
        ),
        (
            "read.src",
            "print(get_shell.host_computer.File(home_dir + \"/shared.txt\").get_content)",
        ),
    ]);
    let worlds = project.path("worlds");
    let options = ["--world-port", &port, "--world-idle-timeout", "2"];

    let write =
        project.run_in_machine_world(&[&["execute", "write.src"][..], &options].concat(), &worlds);
    let read =
        project.run_in_machine_world(&[&["execute", "read.src"][..], &options].concat(), &worlds);

    assert!(write.success(), "{write:?}");
    assert!(read.success(), "{read:?}");
    assert_eq!(read.stdout(), "from the writer\n");
    assert!(worlds.join("worlds.json").exists(), "the world is saved");
}
