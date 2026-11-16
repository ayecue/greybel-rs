//! Programs: launching, a failing launched program, terminal output, input, terminals and the shells they hold.

use greybel_test_support::game::Game;

use crate::support::game::{
    devices_above, join_fixture, routed_ssh_network, setup_player, ssh_network, test_client,
};
use crate::support::parity::{
    Outcome, compare_in, compare_in_real_time, compare_interactive, game_outcomes,
    intrinsics_outcomes,
};
use crate::support::remote::{fixture_account, script_string};
use crate::support::steps::{Step, WorldChange, compare_stepped};

#[tokio::test]
async fn should_match_intrinsics_for_file_and_launch_scripts() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let scripts = [
        "print(get_shell.host_computer.File(\"/bin/ls\").is_binary)",
        "print(get_shell.host_computer.File(\"/bin\").is_folder)",
        "print(get_shell.host_computer.File(\"/bin/ls\").path)",
        "cd(\"/\"); print(get_shell.launch(\"/bin/pwd\"))",
    ];
    let expected = intrinsics_outcomes(&scripts, &user);

    let actual = game_outcomes(&client, &scripts).await;

    assert_eq!(actual, expected);
    client.shutdown();
}

/// What a launched program sees: its `launch_path` is the launching program's path.
#[tokio::test]
async fn should_match_intrinsics_for_launched_programs() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let program = |name: &str, code: &str| {
        format!(
            "pc.touch(home_dir, \"{name}.src\"); pc.File(home_dir + \"/{name}.src\").set_content({}); print sh.build(home_dir + \"/{name}.src\", home_dir) == \"\"",
            script_string(code)
        )
    };
    let paths = [
        "sh = get_shell; pc = sh.host_computer".to_owned(),
        program(
            "grandchild",
            "print \"grandchild \" + launch_path + \" \" + program_path",
        ),
        program(
            "child",
            "print \"child \" + (launch_path == get_custom_object.root) + \" \" + program_path\nget_shell.launch(home_dir + \"/grandchild\")\nprint \"child after \" + (launch_path == get_custom_object.root)",
        ),
        "get_custom_object.root = program_path; print sh.launch(home_dir + \"/child\"); print \"root \" + (launch_path == program_path)".to_owned(),
    ]
    .join("\n");
    let scripts = [paths.as_str()];
    let expected = intrinsics_outcomes(&scripts, &user);

    let actual = game_outcomes(&client, &scripts).await;

    assert_eq!(actual, expected);
    client.shutdown();
}

/// What a launched program's failure leaves: `launch` returns 1, the launching program goes on, and the custom object is
/// reset. Only what the launching program prints after the failure is compared: the failure's own text differs.
#[tokio::test]
async fn should_match_intrinsics_after_a_launched_program_fails() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let script = format!(
        "sh = get_shell; pc = sh.host_computer\npc.touch(home_dir, \"crash.src\"); pc.File(home_dir + \"/crash.src\").set_content({}); sh.build(home_dir + \"/crash.src\", home_dir)\nkept = get_custom_object; kept.mine = 1; launched = sh.launch(home_dir + \"/crash\"); print \"after\"; print launched; print get_custom_object.hasIndex(\"mine\"); print kept.hasIndex(\"mine\"); print get_custom_object.hasIndex(\"done\")",
        script_string("get_custom_object.v = 1\nx = [1][5]\nget_custom_object.done = 1")
    );
    let script = script.as_str();
    let after = |outcome: &Outcome| {
        outcome
            .output
            .as_deref()
            .and_then(|output| output.split_once("after\n"))
            .map(|(_, after)| after.to_owned())
    };
    let expected = intrinsics_outcomes(&[script], &user);

    let actual = game_outcomes(&client, &[script]).await;

    assert_eq!(
        (actual[0].failed, after(&actual[0])),
        (expected[0].failed, after(&expected[0]))
    );
    assert_eq!(after(&expected[0]).as_deref(), Some("1\n0\n1\n0"));
    client.shutdown();
}

/// What the terminal shows of a program's output, which the game sends in batches: a replacing print replaces all the
/// terminal shows with the batch from it on.
#[tokio::test]
async fn should_match_intrinsics_for_what_the_terminal_shows() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let scripts = [
        "print \"one\"; print(\"two\", true); print \"three\"; print(\"four\", 1); print(\"five\", 0)",
        "print(\"one\", true); print \"two\"; wait(0.1); print(\"three\", true); print \"four\"",
        "print \"one\"; wait(0.1); print \"two\"; print(\"three\", true)",
        "print \"one\"; clear_screen; print \"two\"",
        "clear_screen; clear_screen",
        "sh = get_shell; pc = sh.host_computer; pc.touch(home_dir, \"child.src\"); pc.File(home_dir + \"/child.src\").set_content(\"print 2\"); sh.build(home_dir + \"/child.src\", home_dir); print(1, true); sh.launch(home_dir + \"/child\"); print 3",
    ];
    let expected = intrinsics_outcomes(&scripts, &user);

    let actual = game_outcomes(&client, &scripts).await;

    assert_eq!(actual, expected);
    client.shutdown();
}

/// A router or switch between a remote computer and its main router stops starting after the script's terminal on that
/// computer exited: `MainClass` took the terminal's shell off the pending shells when `exit` ended it, so
/// `Computer.IsNetworkActive` no longer goes through that computer and `get_router` still returns the router.
#[tokio::test]
async fn should_match_intrinsics_for_a_device_that_stops_behind_an_exited_terminal() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let network = routed_ssh_network(&client).await;
    let account = fixture_account(&network, "ssh", "ssh", false);
    let host = account.address;
    let port = account.port;
    let password = account.root_password;
    let snapshot = join_fixture(&client, &network).await;
    let devices = devices_above(&snapshot, &host);

    assert!(!devices.is_empty());

    // The firewalls on the way would refuse the connection. The next player setup puts them back.
    for device in &devices {
        client.allow_firewall(device).await.unwrap();
    }

    let snapshot = client.network_state().await.unwrap();
    let script = format!(
        r#"remote = get_shell.connect_service("{host}", {port}, "root", {})
print "terminal " + remote.start_terminal
print "running " + typeof(get_router)
user_input("")
print "stopped " + typeof(get_router)"#,
        script_string(&password)
    );
    let outcome = compare_stepped(
        &client,
        &user,
        &snapshot,
        &script,
        &[
            Step::new(Vec::new(), "exit"),
            Step::new(
                devices
                    .iter()
                    .map(|device| WorldChange::BreakKernel {
                        local_ip: device.clone(),
                    })
                    .collect(),
                "",
            ),
        ],
    )
    .await;

    assert_eq!(
        outcome.output.as_deref(),
        Some("terminal 1\nrunning router\nstopped router"),
        "{outcome:?}"
    );
    client.shutdown();
}

/// Line, masked and any-key input, a queued key returned once by `poll_input`, and `exit(msg)` ending the program.
#[tokio::test]
async fn should_match_intrinsics_for_interactive_input_and_exit() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let snapshot = client.network_state().await.unwrap();
    let script = r#"print "start"
print "ready"
wait(0.2)
print user_input("Line: ")
print "ready"
wait(0.2)
print user_input("Secret: ", 1)
print "ready"
wait(0.2)
print user_input("Key: ", 0, 1)
print poll_input
print "ready"
wait(0.3)
print poll_input
print poll_input
print "ready"
wait(0.2)
print user_input("History: ", 0, 0, 1, 0)
print "ready"
wait(0.2)
print user_input("Quiet key: ", 0, 1, 1, 0)
exit("done")
print "unreachable""#;
    let inputs = ["hello", "secret", "x", "p", "history", "q"].map(str::to_owned);

    compare_interactive(&client, &user, &snapshot, script, &inputs).await;
    client.shutdown();
}

/// `start_terminal` runs the shell's terminal in the program's place, as `launch` runs a program
/// (`ShellIntrinsics.start_terminal`): the computer's `/bin/bash` when that is a program, the game's own bash otherwise,
/// as the shell's user from their home folder, both paths the program's and a new custom object. A `cd` in it moves the
/// terminal, which `exit` ends; the program goes on, and a launched one without its process.
#[tokio::test]
async fn should_match_intrinsics_for_start_terminal() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let network = ssh_network(&client).await;
    let account = fixture_account(&network, "ssh", "ssh", true);
    let port = account.port;
    let root_password = account.root_password;

    let snapshot = join_fixture(&client, &network).await;
    let address = snapshot.public_ip.clone();
    let processes = r#"processes = function(computer)
    names = []
    for line in computer.show_procs.split(char(10))[1:]
        names.push(line.split(" ")[-1])
    end for
    return names.join(",")
end function
"#;
    let local = format!(
        r#"{processes}print "before " + program_path
get_custom_object.x = 1
print "after " + get_shell.start_terminal
print program_path + " " + launch_path + " " + current_path + " " + active_user
print get_custom_object.hasIndex("x")
print processes(get_shell.host_computer)
unbound = @get_shell.start_terminal
result = unbound
print "unbound " + typeof(result)"#
    );
    let remote = format!(
        r#"s = get_shell.connect_service("{address}", {port}, "root", "{root_password}")
print "remote " + s.start_terminal
print "back " + program_path + " " + current_path
pc = s.host_computer
pc.touch("/root", "bash.src")
pc.File("/root/bash.src").set_content("print ""custom "" + active_user + "" "" + current_path + "" "" + program_path + "" "" + launch_path + "" "" + params.len + "" "" + home_dir" + char(10) + "print user_input(""in: "")")
print s.build("/root/bash.src", "/bin")
print "custom " + s.start_terminal
pc.File("/bin/bash").delete
print "deleted " + s.start_terminal"#
    );
    let child = format!(
        "{processes}print \"child \" + get_shell.start_terminal\nprint \"child procs \" + processes(get_shell.host_computer)"
    );
    let launched = format!(
        r#"{processes}pc = get_shell.host_computer
pc.touch(home_dir, "child.src")
pc.File(home_dir + "/child.src").set_content({})
get_shell.build(home_dir + "/child.src", home_dir)
print "launch " + get_shell.launch(home_dir + "/child")
print "parent procs " + processes(pc)"#,
        child
            .lines()
            .map(script_string)
            .collect::<Vec<_>>()
            .join(" + char(10) + ")
    );
    let cases = [
        (local, vec!["whoami", "pwd", "cd /bin", "pwd", "exit"]),
        (
            remote,
            vec!["whoami", "pwd", "exit", "typed", "whoami", "exit"],
        ),
        (launched, vec!["whoami", "exit"]),
    ];

    for (script, inputs) in cases {
        let inputs: Vec<String> = inputs.into_iter().map(str::to_owned).collect();

        compare_interactive(&client, &user, &snapshot, &script, &inputs).await;
    }

    client.shutdown();
}

/// A command a terminal runs may close its own process (`ComputerIntrinsics.close_program`): its program stops, and
/// the terminal goes on. It still can't close the process of the script that started the terminal, which belongs to
/// the script, nor is that refusal on another computer, where the same number is another process.
#[tokio::test]
async fn should_match_intrinsics_for_a_terminal_command_closing_its_own_process() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let network = ssh_network(&client).await;
    let account = fixture_account(&network, "ssh", "ssh", true);
    let port = account.port;
    let root_password = account.root_password;

    let snapshot = join_fixture(&client, &network).await;
    let command = r#"pc = get_shell.host_computer
for line in pc.show_procs.split(char(10))[1:]
    parts = line.split(" ")
    if parts[-1] == "myprogram" then print "script " + pc.close_program(parts[1].to_int)
end for
for line in pc.show_procs.split(char(10))[1:]
    parts = line.split(" ")
    if parts[-1] == "selfclose" then
        print "self " + pc.close_program(parts[1].to_int)
        print "still running"
    end if
end for
print "end"
"#;
    let install = |shell: &str, folder: &str| {
        format!(
            r#"pc = {shell}.host_computer
pc.touch("{folder}", "selfclose.src")
pc.File("{folder}/selfclose.src").set_content({})
{shell}.build("{folder}/selfclose.src", "{folder}")
"#,
            command
                .lines()
                .map(script_string)
                .collect::<Vec<_>>()
                .join(" + char(10) + ")
        )
    };
    let local = format!(
        "{}print \"terminal \" + get_shell.start_terminal\nprint \"done\"",
        install("get_shell", &format!("/home/{user}"))
    );
    let remote = format!(
        "s = get_shell.connect_service(\"{}\", {}, \"root\", {})\n{}print \"terminal \" + s.start_terminal\nprint \"done\"",
        snapshot.public_ip,
        port,
        script_string(&root_password),
        install("s", "/root")
    );
    let inputs: Vec<String> = ["selfclose", "pwd", "exit"]
        .into_iter()
        .map(str::to_owned)
        .collect();

    for script in [local, remote] {
        compare_interactive(&client, &user, &snapshot, &script, &inputs).await;
    }

    client.shutdown();
}

/// Native GUI programs, missing native programs, and the terminal/RAM process guards.
#[tokio::test]
async fn should_match_intrinsics_for_native_and_resource_launch_errors() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let snapshot = client.network_state().await.unwrap();
    let missing = "print get_shell.launch(\"/usr/bin/Chat.exe\"); print get_shell.start_terminal(\"/usr/bin/Chat.exe\"); print get_shell.launch(\"/usr/bin/Notepad.exe\"); print get_shell.start_terminal(\"/usr/bin/Notepad.exe\"); print start_terminal({}, home_dir)".to_owned();

    compare_in(&client, &user, &snapshot, &[missing]).await;

    for fixture in ["low_ram", "terminal_limit"] {
        let user = setup_player(&client).await;

        client.configure_computer("", fixture).await.unwrap();

        let snapshot = client.network_state().await.unwrap();
        let script = if fixture == "low_ram" {
            "print get_shell.host_computer.show_procs; print get_shell.launch(\"/bin/ScanLan.exe\"); print get_shell.start_terminal(\"/bin/ScanLan.exe\")".to_owned()
        } else {
            "print get_shell.launch(\"/bin/ScanLan.exe\"); print get_shell.start_terminal(\"/bin/ScanLan.exe\")".to_owned()
        };

        compare_in(&client, &user, &snapshot, &[script]).await;
    }

    let user = setup_player(&client).await;

    client.configure_computer("", "gui_program").await.unwrap();

    let snapshot = client.network_state().await.unwrap();
    let gui = "print get_shell.launch(\"/usr/bin/Notepad.exe\"); print get_shell.start_terminal(\"/usr/bin/Notepad.exe\")".to_owned();

    compare_in(&client, &user, &snapshot, &[gui]).await;

    let user = setup_player(&client).await;

    client.configure_computer("", "chat_program").await.unwrap();

    let snapshot = client.network_state().await.unwrap();

    compare_interactive(
        &client,
        &user,
        &snapshot,
        "print get_shell.start_terminal(\"/usr/bin/Chat.exe\")",
        &["exit".to_owned()],
    )
    .await;
    client.shutdown();
}

/// A self-launching program reaches the game's 32 pending-VM limit without an unbounded loop.
#[tokio::test]
async fn should_match_intrinsics_for_the_launch_stack_limit() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let snapshot = client.network_state().await.unwrap();
    let source = script_string(
        "n = params[0].to_int\nif not get_shell.launch(program_path, str(n + 1)) then print \"refused at \" + n",
    );
    let script = format!(
        "pc = get_shell.host_computer; pc.touch(home_dir, \"nested.src\"); pc.File(home_dir + \"/nested.src\").set_content({source}); get_shell.build(home_dir + \"/nested.src\", home_dir); print get_shell.launch(home_dir + \"/nested\", \"0\")"
    );

    let outcomes = compare_in(&client, &user, &snapshot, &[script]).await;

    assert!(
        outcomes[0]
            .output
            .as_deref()
            .is_some_and(|output| output.contains("refused at 32"))
    );
    client.shutdown();
}

/// Chat is the native program `launch` is allowed to start, after which it receives terminal input.
#[tokio::test]
async fn should_match_intrinsics_for_launching_chat() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;

    client.configure_computer("", "chat_program").await.unwrap();

    let snapshot = client.network_state().await.unwrap();

    compare_interactive(
        &client,
        &user,
        &snapshot,
        "print get_shell.launch(\"/usr/bin/Chat.exe\")",
        &["exit".to_owned()],
    )
    .await;
    client.shutdown();
}

/// A child launched on a remote computer holds that computer's shell (`MainClass.Main`), so the player's network
/// needs that computer available too (`Computer.IsNetworkActive`): once the child deletes the computer's
/// `/lib/init.so` (`Computer.CheckInicio`), its `get_router` finds no internet connection, and the program that launched it, which
/// holds only its own shell, still finds its router.
#[tokio::test]
async fn should_match_intrinsics_for_a_held_shell_whose_device_becomes_unavailable() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let network = ssh_network(&client).await;
    let account = fixture_account(&network, "ssh", "ssh", true);
    let port = account.port;
    let root_password = account.root_password;

    let snapshot = join_fixture(&client, &network).await;
    let child = r#"print "child " + typeof(get_router)
print get_shell.host_computer.File("/lib/init.so").delete
print get_shell.host_computer.is_network_active
print "child " + typeof(get_router)"#;
    let script = format!(
        r#"remote = get_shell.connect_service("{}", {}, "root", {})
pc = remote.host_computer
pc.touch("/root", "child.src")
pc.File("/root/child.src").set_content({})
remote.build("/root/child.src", "/root")
print "launch " + remote.launch("/root/child")
print "parent " + typeof(get_router)"#,
        snapshot.public_ip,
        port,
        script_string(&root_password),
        child
            .lines()
            .map(script_string)
            .collect::<Vec<_>>()
            .join(" + char(10) + "),
    );

    compare_in_real_time(&client, &user, &snapshot, &[script]).await;
    client.shutdown();
}

/// A terminal started on a shell makes the program's tree hold that shell for the rest of its life, after the terminal
/// ended too (`ShellIntrinsics.start_terminal`, `HelperScript.AddPendingShell`): once the shell's computer stops
/// starting (`Computer.CheckInicio`), the player's computer has no network for the program (`Computer.IsNetworkActive`).
#[tokio::test]
async fn should_match_intrinsics_for_a_shell_held_by_its_terminal() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let network = ssh_network(&client).await;
    let account = fixture_account(&network, "ssh", "ssh", true);
    let port = account.port;
    let root_password = account.root_password;

    let snapshot = join_fixture(&client, &network).await;
    let script = format!(
        r#"s = get_shell.connect_service("{}", {}, "root", {})
print "before " + typeof(get_router)
print "terminal " + s.start_terminal
print "kept " + typeof(get_router)
print s.host_computer.File("/lib/init.so").delete
print "unavailable " + typeof(get_router)"#,
        snapshot.public_ip,
        port,
        script_string(&root_password),
    );

    compare_interactive(&client, &user, &snapshot, &script, &["exit".to_owned()]).await;
    client.shutdown();
}
