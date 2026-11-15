use greybel_test_support::future::block_on;
use greyhack_world::{PlayerOptions, World};
use greyhack_world_libraries::ReverseShell;

use crate::support::rshell::prepare_rshell;
use crate::support::save_copy;
use crate::support::scripts::{run_code_in, run_on_host};

#[test]
fn should_register_a_reverse_shell_and_return_it_from_the_server() {
    let mut world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let (player, _) = prepare_rshell(&mut world);
    let public_ip = block_on(world.modules.core.networks.topology.public_ip(player)).unwrap();
    let code = format!(
        "meta = include_lib(\"/lib/metaxploit.so\")\nprint meta.rshell_client(\"{public_ip}\", 1222, \"client\")\nshells = meta.rshell_server\nprint shells.len\nprint typeof(shells[0])"
    );

    let (output, error, _) = run_on_host(&code, world, true);

    assert_eq!(error, None);
    assert_eq!(output, "1\n1\nshell");
}

#[test]
fn should_give_each_client_process_its_own_shell() {
    let mut world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let (player, _) = prepare_rshell(&mut world);
    let public_ip = block_on(world.modules.core.networks.topology.public_ip(player)).unwrap();
    let code = format!(
        "meta = include_lib(\"/lib/metaxploit.so\")\nmeta.rshell_client(\"{public_ip}\", 1222, \"one\")\nmeta.rshell_client(\"{public_ip}\", 1222, \"two\")\nshells = meta.rshell_server\nprint shells.len\nprint shells[0] == shells[1]\nprint meta.rshell_server[0] == shells[0]"
    );

    let (output, error, _) = run_on_host(&code, world, true);

    assert_eq!(error, None);
    assert_eq!(output, "2\n0\n1");
}

/// Their computers are one object: the game's `AddComputer` compares the access, not the shell's process.
#[test]
fn should_give_the_shells_of_one_client_computer_one_host_computer() {
    let mut world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let (player, _) = prepare_rshell(&mut world);
    let public_ip = block_on(world.modules.core.networks.topology.public_ip(player)).unwrap();
    let code = format!(
        "meta = include_lib(\"/lib/metaxploit.so\")\nmeta.rshell_client(\"{public_ip}\", 1222, \"one\")\nmeta.rshell_client(\"{public_ip}\", 1222, \"two\")\nshells = meta.rshell_server\nprint shells[0].host_computer == shells[1].host_computer\nprint shells[0].host_computer.public_ip == \"{public_ip}\""
    );

    let (output, error, _) = run_on_host(&code, world, true);

    assert_eq!(error, None);
    assert_eq!(output, "1\n1");
}

#[test]
fn should_drop_a_reverse_shell_whose_computer_rebooted_after_it() {
    let mut world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let (player, router) = prepare_rshell(&mut world);
    let made = world.handle.time_seconds();

    block_on(world.modules.core.computers.processes.reboot(player, false)).unwrap();
    block_on(world.tick(10 * 60)).unwrap();

    // A shell made before the reboot whose client process runs again.
    let pid = block_on(
        world
            .modules
            .core
            .computers
            .processes
            .spawn_process(player, "test", "client", None),
    )
    .unwrap();
    let shell = ReverseShell {
        computer: player,
        port: 1222,
        user: "test".to_owned(),
        pid,
        creation_second: made,
    };

    block_on(
        world
            .modules
            .libraries
            .metaxploit
            .insert_reverse_shell(router, &shell),
    )
    .unwrap();

    let code = "print include_lib(\"/lib/metaxploit.so\").rshell_server.len";
    let (output, error, _) = run_on_host(code, world, true);

    assert_eq!(error, None);
    assert_eq!(output, "0");
}

#[test]
fn should_reject_a_reserved_reverse_shell_process_name() {
    let code = "print include_lib(\"/lib/metaxploit.so\").rshell_client(\"1.1.1.1\", 1222, \"kernel_task\")";

    let (output, error) = run_code_in(code, true);

    assert_eq!(error, None);
    assert_eq!(output, "Error: kernel_task is a reserved process name");
}

#[tokio::test]
async fn should_preserve_reverse_shells_when_saving_and_loading() {
    let mut world = World::new(&PlayerOptions::default()).await.unwrap();
    let (_, router) = prepare_rshell(&mut world);
    let player = world.handle.player();
    let pid = world
        .modules
        .core
        .computers
        .processes
        .spawn_process(player, "test", "client", None)
        .await
        .unwrap();

    world
        .modules
        .libraries
        .metaxploit
        .insert_reverse_shell(
            router,
            &ReverseShell {
                computer: player,
                port: 1222,
                user: "test".to_owned(),
                pid,
                creation_second: world.handle.time_seconds(),
            },
        )
        .await
        .unwrap();

    let database = save_copy(&mut world).await;

    let restored = World::load(&database).await.unwrap();

    let shells = restored
        .modules
        .libraries
        .metaxploit
        .reverse_shells(router)
        .await
        .unwrap();

    assert_eq!(shells.len(), 1);
    assert_eq!(shells[0].port, 1222);
    assert_eq!(shells[0].pid, pid);
}
