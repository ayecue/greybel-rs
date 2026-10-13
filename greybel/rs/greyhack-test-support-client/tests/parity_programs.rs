//! Programs: launching, a failing launched program, terminal output, input, terminals and the shells they hold.

use greybel_test_support::game::Game;

use crate::support::game::{join_network, setup_player, test_client};
use crate::support::parity::{
    Outcome, compare_in_real_time, compare_interactive, game_outcomes, intrinsics_outcomes,
};
use crate::support::remote::{forwarded_root, script_string, service_behind_devices};
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
        "sh = get_shell; pc = sh.host_computer; pc.touch(home_dir, \"child.src\"); pc.File(home_dir + \"/child.src\").set_content(\"print 2\"); sh.build(home_dir + \"/child.src\", home_dir); print(1, true); sh.launch(home_dir + \"/child\"); print 3",
    ];
    let expected = intrinsics_outcomes(&scripts, &user);

    let actual = game_outcomes(&client, &scripts).await;

    assert_eq!(actual, expected);
    client.shutdown();
}

/// A script whose terminal runs on a remote computer loses its network once a router or switch between that computer
/// and its main router stops starting (`Computer.IsNetworkActive`, `NetworkLan.ReversePing`).
#[tokio::test]
async fn should_match_intrinsics_for_a_terminal_behind_a_device_that_stops() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let service = service_behind_devices(&client, &["ssh"]).await;
    let host = service.address;
    let port = service.port;
    let password = service.root_password;
    let devices = service.devices;

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

    // `PlayerIntrinsics.get_router` returns "get_router: No internet connection" instead of raising it.
    assert_eq!(
        outcome.output.as_deref(),
        Some("terminal 1\nrunning router\nstopped string"),
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
exit("done")
print "unreachable""#;
    let inputs = ["hello", "secret", "x", "p"].map(str::to_owned);

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
    let network = client
        .find_new_network("Supermercados", "ssh")
        .await
        .unwrap();

    join_network(&client, &network.public_ip).await;

    let snapshot = client.network_state().await.unwrap();
    let (port, root_password) = forwarded_root(&snapshot, "ssh");
    let address = snapshot.public_ip.to_string();
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
    let network = client
        .find_new_network("Supermercados", "ssh")
        .await
        .unwrap();

    join_network(&client, &network.public_ip).await;

    let snapshot = client.network_state().await.unwrap();
    let (port, root_password) = forwarded_root(&snapshot, "ssh");
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

/// A child launched on a remote computer holds that computer's shell (`MainClass.Main`), so the player's network
/// needs that computer available too (`Computer.IsNetworkActive`): once the child deletes the computer's
/// `/lib/init.so` (`Computer.CheckInicio`), its `get_router` finds no internet connection, and the program that launched it, which
/// holds only its own shell, still finds its router.
#[tokio::test]
async fn should_match_intrinsics_for_a_held_shell_whose_device_becomes_unavailable() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let network = client
        .find_new_network("Supermercados", "ssh")
        .await
        .unwrap();

    join_network(&client, &network.public_ip).await;

    let snapshot = client.network_state().await.unwrap();
    let (port, root_password) = forwarded_root(&snapshot, "ssh");
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
    let network = client
        .find_new_network("Supermercados", "ssh")
        .await
        .unwrap();

    join_network(&client, &network.public_ip).await;

    let snapshot = client.network_state().await.unwrap();
    let (port, root_password) = forwarded_root(&snapshot, "ssh");
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
