use duplex_test_support::local::run_local;
use futures_util::future::join;
use greybel_interpreter::RunResult;
use greybel_test_support::future::block_on;
use greybel_test_support::greyhack::connect;
use greybel_test_support::interpreter::{constant_clock, inline, snapshot_options};
use greyhack_world::{PlayerOptions, ProcessFailure, World};
use greyhack_world_operations::WorldServer;
use greyscript_intrinsics::GreyHackInterpreter;

use crate::support::processes::run_with_child;
use crate::support::save_copy;
use crate::support::scripts::run_on_host;

#[test]
fn should_reject_a_launch_when_ram_is_full() {
    let world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let player = world.handle.player();

    while block_on(
        world
            .modules
            .core
            .computers
            .processes
            .check_process_capacity(player, 25.0, false),
    )
    .unwrap()
    .is_ok()
    {
        block_on(
            world
                .modules
                .core
                .computers
                .processes
                .spawn_process(player, "test", "filler", None),
        )
        .unwrap();
    }

    let (output, error, _) = run_on_host("print get_shell.launch(\"/bin/ls\")", world, true);

    assert_eq!(error, None);
    assert!(output.ends_with("\n0"), "{output}");
    assert!(!output.contains("command not found"), "{output}");
}

#[test]
fn should_refuse_a_terminal_past_the_terminal_limit() {
    let world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let player = world.handle.player();

    for index in 0..15 {
        block_on(world.modules.core.computers.processes.spawn_terminal(
            player,
            "test",
            &format!("terminal{index}"),
        ))
        .unwrap();
    }

    let error = block_on(
        world
            .modules
            .core
            .computers
            .processes
            .check_process_capacity(player, 25.0, true),
    )
    .unwrap()
    .unwrap_err();

    assert_eq!(error, ProcessFailure::TooManyTerminals);
}

/// `launch` takes only text parameters (`ShellIntrinsics`), and its invalid arguments fail the script before the
/// terminal limit is looked at; Grey Hack fails `launch("/bin/ls", null)` with this error.
#[test]
fn should_reject_launch_parameters_that_arent_text() {
    // Arrange
    let world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let player = world.handle.player();

    for index in 0..15 {
        block_on(world.modules.core.computers.processes.spawn_terminal(
            player,
            "test",
            &format!("terminal{index}"),
        ))
        .unwrap();
    }

    // Act
    let (output, error, _) = run_on_host("print get_shell.launch(\"/bin/ls\", null)", world, true);

    // Assert
    assert_eq!(
        (output.trim_end(), error.as_deref()),
        ("launch: Invalid arguments", None)
    );
    assert!(
        !output.contains("Maximum number of Terminals in execution reached."),
        "{output}"
    );
}

#[tokio::test]
async fn should_preserve_terminal_process_flags() {
    let mut world = World::new(&PlayerOptions::default()).await.unwrap();
    let player = world.handle.player();

    world
        .modules
        .core
        .computers
        .processes
        .spawn_terminal(player, "test", "terminal")
        .await
        .unwrap();

    let database = save_copy(&mut world).await;

    let restored = World::load(&database).await.unwrap();

    assert!(
        restored
            .modules
            .core
            .computers
            .processes
            .processes(player)
            .await
            .unwrap()
            .iter()
            .any(|process| process.name == "terminal" && process.is_terminal)
    );
}

/// Closing another runtime's process through the shared game server terminates its interpreter at the current
/// instruction.
/// On the player's computer a script can't close the process of the program its terminal started
/// (`ComputerIntrinsics.close_program`).
#[test]
fn should_refuse_to_close_the_scripts_own_process() {
    let world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let code = r#"pc = get_shell.host_computer
name = program_path.split("/")[-1]
for line in pc.show_procs.split(char(10))
  cols = line.split(" ")
  if cols[-1] == name then print pc.close_program(cols[1].to_int)
end for
print "went on""#;

    let (output, error, _) = run_on_host(code, world, true);

    assert_eq!(error, None);
    assert_eq!(
        output,
        "Unable to close this process. The process belongs to this script.\nwent on"
    );
}

#[test]
fn should_end_a_program_whose_process_another_program_closed() {
    run_local(async {
        let server = WorldServer::new(
            World::new(&PlayerOptions::default()).await.unwrap(),
            constant_clock(),
        )
        .await
        .unwrap();
        let user = PlayerOptions::default().user;
        let (_, looping_options) = snapshot_options();
        let (closing_lines, closing_options) = snapshot_options();
        let mut looping = GreyHackInterpreter::start(
            &connect(&server),
            &user,
            looping_options,
            inline("while true\n  current_date\nend while"),
        )
        .await
        .expect("the looping program starts");
        let player = server.world().await.player();
        let pid = server
            .world()
            .await
            .modules
            .core
            .computers
            .processes
            .processes(player)
            .await
            .unwrap()
            .into_iter()
            .find(|process| process.name == "script")
            .map(|process| process.pid)
            .expect("the looping program's process");
        let mut closing = GreyHackInterpreter::start(
            &connect(&server),
            &user,
            closing_options,
            inline(&format!(
                "print get_shell.host_computer.close_program({pid})"
            )),
        )
        .await
        .expect("the closing program starts");

        let (looped, closed) = join(looping.run(), closing.run()).await;

        assert!(matches!(looped, Ok(RunResult::Complete)), "{looped:?}");
        assert!(matches!(closed, Ok(RunResult::Complete)), "{closed:?}");
        assert_eq!(closing_lines.text(), "1");

        let running = server
            .world()
            .await
            .modules
            .core
            .computers
            .processes
            .process(player, pid)
            .await
            .unwrap()
            .is_some();

        assert!(!running);
    });
}

/// Launching the same binary twice reuses its cached code entry and does not increase the VM's code count.
#[test]
fn should_reuse_the_code_of_a_binary_launched_again() {
    run_local(async {
        let server = WorldServer::new(
            World::new(&PlayerOptions::default()).await.unwrap(),
            constant_clock(),
        )
        .await
        .unwrap();
        let user = PlayerOptions::default().user;
        let build = "sh = get_shell\npc = sh.host_computer\npc.touch(home_dir, \"child.src\")\npc.File(home_dir + \"/child.src\").set_content(\"f = function\n  return 1\nend function\nprint f\")\nsh.build(home_dir + \"/child.src\", home_dir)\n";
        let mut code_counts = Vec::new();

        for launches in [1, 2] {
            let (lines, options) = snapshot_options();
            let launch = "sh.launch(home_dir + \"/child\")\n".repeat(launches);
            let mut program = GreyHackInterpreter::start(
                &connect(&server),
                &user,
                options,
                inline(&format!("{build}{launch}")),
            )
            .await
            .expect("the program starts")
            .with_admin(true);

            program.run().await.expect("runs");

            assert_eq!(lines.text(), vec!["1"; launches].join("\n"));
            code_counts.push(
                program
                    .interpreter()
                    .vm()
                    .expect("started")
                    .program()
                    .code_count(),
            );
        }

        assert_eq!(code_counts[0], code_counts[1]);
    });
}

/// A closure stored in the shared custom object resolves outer names in the child's globals and `globals` in the
/// parent's globals, matching the game.
#[test]
fn should_call_a_function_a_launched_program_stored_in_the_custom_object() {
    // Arrange
    let child = "x = \"child\"\nget_custom_object.f = function()\n  return x + \" \" + globals.x\nend function";
    let parent =
        "x = \"parent\"\nsh.launch(home_dir + \"/child\")\nf = @get_custom_object.f\nprint f";

    // Act
    let (output, error) = run_with_child(child, parent);

    // Assert
    assert_eq!(error, None);
    assert_eq!(output.lines().last(), Some("child parent"), "{output}");
}

/// Parent-context values remain GC roots while the child context triggers garbage collection.
#[test]
fn should_keep_the_values_of_the_launching_program_while_the_launched_one_collects() {
    // Arrange
    let child = "for i in range(0, 30000)\n  m = {\"a\": [i]}\nend for";
    let parent =
        "kept = {\"name\": [\"alive\"]}\nsh.launch(home_dir + \"/child\")\nprint kept.name[0]";

    // Act
    let (output, error) = run_with_child(child, parent);

    // Assert
    assert_eq!(error, None);
    assert_eq!(output.lines().last(), Some("alive"), "{output}");
}

/// All contexts in a launch tree share the custom object; a child crash resets it (`MainClass`'s `scriptCrash`). `launch`
/// returns 1 either way.
#[test]
fn should_drop_the_custom_object_when_a_launched_program_crashes() {
    // Arrange
    let sharing = "get_custom_object.v = 1";
    let crashing = "get_custom_object.v = 1\nx = [1][5]";
    let parent = "print sh.launch(home_dir + \"/child\")\nprint get_custom_object.hasIndex(\"v\")";

    // Act
    let (shared, shared_error) = run_with_child(sharing, parent);
    let (crashed, crashed_error) = run_with_child(crashing, parent);

    // Assert
    assert_eq!((shared_error, crashed_error), (None, None));

    let shared_end: Vec<&str> = shared.lines().rev().take(2).collect();
    let crashed_end: Vec<&str> = crashed.lines().rev().take(2).collect();

    assert_eq!(shared_end, ["1", "1"], "{shared}");
    assert_eq!(crashed_end, ["0", "1"], "{crashed}");
}

/// Terminating a child context while its own child is running unwinds both contexts; the root then resumes with its
/// values.
#[test]
fn should_go_on_after_a_launched_program_ends_while_it_launched_another() {
    // Arrange
    let grandchild = "for line in get_shell.host_computer.show_procs.split(char(10))\n  cols = line.split(\" \")\n  if cols[-1] == \"child\" then get_shell.host_computer.close_program(cols[1].to_int)\nend for\nwhile true\n  yield\nend while";
    let child = "get_shell.launch(home_dir + \"/grandchild\")\nprint \"child went on\"";
    let parent = format!(
        "pc.touch(home_dir, \"grandchild.src\")\npc.File(home_dir + \"/grandchild.src\").set_content(\"{}\")\nsh.build(home_dir + \"/grandchild.src\", home_dir)\nkept = [\"alive\"]\nsh.launch(home_dir + \"/child\")\nprint kept[0]",
        grandchild.replace('"', "\"\"")
    );

    // Act
    let (output, error) = run_with_child(child, &parent);

    // Assert
    assert_eq!(error, None);
    assert_eq!(output.lines().last(), Some("alive"), "{output}");
    assert!(!output.contains("child went on"), "{output}");
}

/// A launched program's `launch_path` is the path of the program that launched it (`HelperScript.AddPendingVM` sets
/// `launchPath = pathProgram` before switching to the launched program), and its `program_path` is its own.
#[test]
fn should_give_a_launched_program_the_launching_programs_path_as_its_launch_path() {
    // Arrange
    let grandchild = "print \"grandchild \" + launch_path + \" \" + program_path";
    let child = "print \"child \" + launch_path + \" \" + program_path\nget_shell.launch(home_dir + \"/grandchild\")\nprint \"child after \" + launch_path + \" \" + program_path";
    let parent = format!(
        "pc.touch(home_dir, \"grandchild.src\")\npc.File(home_dir + \"/grandchild.src\").set_content(\"{}\")\nsh.build(home_dir + \"/grandchild.src\", home_dir)\nroot = program_path\nsh.launch(home_dir + \"/child\")\nprint \"root \" + (launch_path == root) + \" \" + (program_path == root)",
        grandchild.replace('"', "\"\"")
    );

    // Act
    let (output, error) = run_with_child(child, &parent);

    // Assert
    assert_eq!(error, None);

    let lines: Vec<&str> = output.lines().filter(|line| !line.is_empty()).collect();

    assert_eq!(
        lines,
        [
            "child /home/test/script /home/test/child",
            "grandchild /home/test/child /home/test/grandchild",
            "child after /home/test/script /home/test/child",
            "root 1 1",
        ]
    );
}
