use greybel_test_support::future::block_on;
use greyhack_world::World;

use crate::support::scripts::run_on_host;
use crate::support::system_log::{
    ADDRESS, UNSEC_VALUE, connect_script, entries, public_ip, router_computer, run_typing,
    world_with_service, world_with_shell_weakness,
};

/// A login leaves `open` on the computer it reaches, and the shell it opens leaves `shell` there, `bounce` on the
/// player's computer and `bounce` on the player's router (`PlayerUtils.ConnectToService`,
/// `PlayerUtils.UpdateBounceLog`).
#[test]
fn should_log_a_connection_on_the_target_the_player_and_the_router() {
    // Arrange
    let (world, target) = world_with_service();
    let player = world.handle.player();
    let player_ip = public_ip(&world, player);
    let router = router_computer(&world, player);
    let local_ip = block_on(world.modules.core.networks.topology.computer(player))
        .unwrap()
        .unwrap()
        .local_ip;
    let target_ip = public_ip(&world, target.computer);
    let script = connect_script(
        &target.address,
        target.port,
        &target.service,
        &target.user,
        &target.password,
    );

    // Act
    let (output, error, world) = run_on_host(&script, world, true);

    // Assert
    assert_eq!((output.trim_end(), error.as_deref()), ("shell", None));
    assert_eq!(
        entries(&world, target.computer),
        [
            (
                0,
                player_ip.clone(),
                i64::from(target.port),
                String::new(),
                String::new()
            ),
            (
                2,
                player_ip.clone(),
                i64::from(target.port),
                String::new(),
                String::new()
            ),
        ]
    );
    assert_eq!(
        entries(&world, player),
        [(4, player_ip.clone(), 0, String::new(), target_ip.clone())]
    );
    assert_eq!(
        entries(&world, router),
        [(4, local_ip, 0, String::new(), target_ip)]
    );
}

/// A wrong password logs nothing anywhere.
#[test]
fn should_log_nothing_for_a_wrong_password() {
    // Arrange
    let (world, target) = world_with_service();
    let player = world.handle.player();
    let script = connect_script(
        &target.address,
        target.port,
        &target.service,
        &target.user,
        "wrong",
    );

    // Act
    let (_, error, world) = run_on_host(&script, world, true);

    // Assert
    assert_eq!(error, None);
    assert_eq!(entries(&world, target.computer), []);
    assert_eq!(entries(&world, player), []);
}

/// Deleting a file on a computer logs `remove` with the file's path and the address the player came from; the
/// player's own shell names the player's own address.
#[test]
fn should_log_a_deleted_file_with_its_path() {
    // Arrange
    let world = block_on(World::new(&greyhack_world::PlayerOptions::default())).expect("a world");
    let player = world.handle.player();
    let player_ip = public_ip(&world, player);
    let script = "pc = get_shell.host_computer\npc.touch(home_dir, \"gone.txt\")\nprint pc.File(home_dir + \"/gone.txt\").delete\nprint pc.File(home_dir + \"/missing.txt\") == null";

    // Act
    let (output, error, world) = run_on_host(script, world, true);

    // Assert
    assert_eq!(error, None);
    assert_eq!(output.trim_end(), "\n1");

    let user = greyhack_world::PlayerOptions::default().user;

    assert_eq!(
        entries(&world, player),
        [(
            3,
            player_ip,
            0,
            format!("/home/{user}/gone.txt"),
            String::new()
        )]
    );
}

/// A file deleted through a remote shell is logged on the remote computer with the address of the computer the
/// shell was opened from.
#[test]
fn should_log_a_file_deleted_through_a_remote_shell() {
    // Arrange
    let (world, target) = world_with_service();
    let player = world.handle.player();
    let player_ip = public_ip(&world, player);
    let script = format!(
        "shell = get_shell.connect_service(\"{}\", {}, \"{}\", \"{}\", \"{}\")\npc = shell.host_computer\npc.touch(\"/home/guest\", \"gone.txt\")\nprint pc.File(\"/home/guest/gone.txt\").delete",
        target.address, target.port, target.user, target.password, target.service
    );

    // Act
    let (_, error, world) = run_on_host(&script, world, true);

    // Assert
    assert_eq!(error, None);
    assert_eq!(
        entries(&world, target.computer).last(),
        Some(&(
            3,
            player_ip,
            0,
            "/home/guest/gone.txt".to_owned(),
            String::new()
        ))
    );
}

/// `net_use` on a service port logs `open` on the computer behind it, naming the metaxploit computer's address and
/// the port; a router session logs nothing (`MetaExploitIntrinsics.net_use`).
#[test]
fn should_log_a_net_session_on_a_service() {
    // Arrange
    let (world, target) = world_with_service();
    let player = world.handle.player();
    let player_ip = public_ip(&world, player);
    let script = format!(
        "mx = include_lib(\"/lib/metaxploit.so\")\nsession = mx.net_use(\"{}\", {})\nprint typeof(session)\nrouter = mx.net_use(\"{}\", 0)\nprint typeof(router)",
        target.address, target.port, target.address
    );

    // Act
    let (output, error, world) = run_on_host(&script, world, true);

    // Assert
    assert_eq!(error, None);
    assert!(output.starts_with("NetSession"), "{output}");
    assert_eq!(
        entries(&world, target.computer),
        [(
            0,
            player_ip,
            i64::from(target.port),
            String::new(),
            String::new()
        )]
    );
    assert_eq!(
        entries(&world, router_computer(&world, target.computer)),
        []
    );
}

/// An overflow that gives a shell logs it like a connection does: `shell` on the target, `bounce` on the metaxploit
/// computer and its router (`MetaLibIntrinsics.overflow`).
#[test]
fn should_log_a_shell_from_an_overflow() {
    // Arrange
    let (world, target) = world_with_shell_weakness();
    let player = world.handle.player();
    let player_ip = public_ip(&world, player);
    let router = router_computer(&world, player);
    let local_ip = block_on(world.modules.core.networks.topology.computer(player))
        .unwrap()
        .unwrap()
        .local_ip;
    let target_ip = public_ip(&world, target.computer);
    let script = format!(
        "mx = include_lib(\"/lib/metaxploit.so\")\nsession = mx.net_use(\"{}\", {})\nlib = session.dump_lib\nshell = lib.overflow(\"{ADDRESS}\", \"{UNSEC_VALUE}\")\nprint typeof(shell)",
        target.address, target.port
    );

    // Act
    let (output, error, world) = run_on_host(&script, world, true);

    // Assert
    assert_eq!(error, None);
    assert!(output.trim_end().ends_with("shell"), "{output}");
    assert_eq!(
        entries(&world, target.computer),
        [
            (
                0,
                player_ip.clone(),
                i64::from(target.port),
                String::new(),
                String::new()
            ),
            (
                2,
                player_ip.clone(),
                i64::from(target.port),
                String::new(),
                String::new()
            ),
        ]
    );
    assert_eq!(
        entries(&world, player),
        [(4, player_ip.clone(), 0, String::new(), target_ip.clone())]
    );
    assert_eq!(
        entries(&world, router),
        [(4, local_ip, 0, String::new(), target_ip)]
    );
}

/// A terminal started on a remote shell and left with `exit` logs `close` on that computer with the address of the
/// computer it was opened from (`HelperScript.CloseRemotePrograms`).
#[test]
fn should_log_a_terminal_left_with_exit() {
    // Arrange
    let (world, target) = world_with_service();
    let player = world.handle.player();
    let player_ip = public_ip(&world, player);
    let script = format!(
        "shell = get_shell.connect_service(\"{}\", {}, \"{}\", \"{}\", \"{}\")\nprint shell.start_terminal\nprint \"back\"",
        target.address, target.port, target.user, target.password, target.service
    );

    // Act
    let (output, error, world) = run_typing(&script, world, &["exit"]);

    // Assert
    assert_eq!(error, None);
    assert!(output.trim_end().ends_with("back"), "{output}");

    let on_target = entries(&world, target.computer);

    assert_eq!(
        on_target.last(),
        Some(&(1, player_ip, 0, String::new(), String::new()))
    );
    assert_eq!(on_target.len(), 3, "{on_target:?}");
}

/// A terminal on the player's own computer closes nothing, and one left by running out of code instead of `exit`
/// closes nothing either.
#[test]
fn should_not_log_a_local_terminal_or_one_that_runs_out_of_code() {
    // Arrange
    let (world, target) = world_with_service();
    let player = world.handle.player();
    let local = "print get_shell.start_terminal";
    let custom = format!(
        "shell = get_shell.connect_service(\"{}\", {}, \"{}\", \"{}\", \"{}\")\npc = shell.host_computer\npc.touch(\"/home/guest\", \"bash.src\")\npc.File(\"/home/guest/bash.src\").set_content(\"print 1\")\nprint shell.build(\"/home/guest/bash.src\", \"/bin\")\nprint shell.start_terminal",
        target.address, target.port, target.user, target.password, target.service
    );

    // Act
    let (_, error, after_local) = run_typing(local, world, &["exit"]);

    // Assert
    assert_eq!(error, None);
    assert_eq!(entries(&after_local, player), []);

    // Act
    let (world, target) = world_with_service();
    let (output, error, after_custom) = run_typing(&custom, world, &[]);

    // Assert
    assert_eq!(error, None);
    assert!(
        output.trim_end().ends_with("1\n1"),
        "the custom bash ran: {output}"
    );
    assert!(
        entries(&after_custom, target.computer)
            .iter()
            .all(|entry| entry.0 != 1),
        "no close entry"
    );
}
