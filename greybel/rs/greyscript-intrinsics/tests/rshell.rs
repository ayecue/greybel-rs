use greybel_test_support::future::block_on;
use greyhack_behaviour::NetworkKind;
use greyhack_world::{PlayerOptions, World};
use greyhack_world_libraries::ReverseShell;

use crate::support::network::{SERVICE_SEED, first_remote_service};
use crate::support::rshell::{prepare_rshell, prepare_rshell_service};
use crate::support::scripts::{run_code_in, run_on_host};
use crate::support::{read_computer, save_copy, world_options};

#[test]
fn should_check_rshell_service_after_losing_the_host_init_file() {
    let world = block_on(World::generate(&world_options(SERVICE_SEED, 1))).expect("a world");
    let target = first_remote_service(&world);
    let code = format!(
        "remote = get_shell.connect_service(\"{}\", {}, \"{}\", \"{}\"); remote.host_computer.File(\"/lib/init.so\").delete; print include_lib(\"/lib/metaxploit.so\").rshell_server",
        target.address, target.port, target.user, target.password
    );

    let (output, error, _) = run_on_host(&code, world, true);

    assert_eq!(error, None);
    assert_eq!(output, "error: service rshelld is not running");
}

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
    let (output, error, world) = run_on_host(code, world, true);

    assert_eq!(error, None);
    assert_eq!(output, "0");
    assert!(
        block_on(world.modules.libraries.metaxploit.reverse_shells(router))
            .unwrap()
            .is_empty()
    );
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

#[test]
fn should_refuse_the_seventeenth_reverse_shell_on_a_port() {
    let mut world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let (player, _) = prepare_rshell(&mut world);
    let public_ip = block_on(world.modules.core.networks.topology.public_ip(player)).unwrap();
    let code = format!(
        "mx = include_lib(\"/lib/metaxploit.so\")\nfor i in range(1, 17)\nprint mx.rshell_client(\"{public_ip}\", 1222, \"client\")\nend for\nprint mx.rshell_server.len"
    );

    let (output, error, _) = run_on_host(&code, world, true);

    assert_eq!(error, None);
    assert_eq!(
        output,
        format!(
            "{}rshell_client: connection refused. The server has reached its limit of 16 connections.\n16",
            "1\n".repeat(16)
        )
    );
}

#[test]
fn should_remove_all_or_selected_reverse_shells_after_validating_the_list() {
    let mut world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let (player, _) = prepare_rshell(&mut world);
    let public_ip = block_on(world.modules.core.networks.topology.public_ip(player)).unwrap();
    let code = format!(
        r#"mx = include_lib("/lib/metaxploit.so")
print mx.rshell_remove
mx.rshell_client("{public_ip}", 1222, "one")
mx.rshell_client("{public_ip}", 1222, "two")
shells = mx.rshell_server
print mx.rshell_remove(1)
print mx.rshell_remove([get_shell])
print mx.rshell_remove([shells[0], 1])
print mx.rshell_server.len
print mx.rshell_remove([shells[0]])
print mx.rshell_remove([shells[0]])
print mx.rshell_server.len
print mx.rshell_remove(null)
print mx.rshell_remove([])
print mx.rshell_server.len
print rshell_remove({{}})"#
    );

    let (output, error, _) = run_on_host(&code, world, true);

    assert_eq!(error, None);
    assert_eq!(
        output,
        "0\nrshell_remove: expected a list of reverse shells or null\nrshell_remove: list must contain only reverse shells\nrshell_remove: list must contain only reverse shells\n2\n1\n0\n1\n1\n0\n0\nnull"
    );
}

#[test]
fn should_check_the_rshell_service_before_the_removal_list_and_forward() {
    let code = "mx = include_lib(\"/lib/metaxploit.so\")\nprint mx.rshell_remove(1)";
    let (output, error, world) = run_on_host(
        code,
        block_on(World::new(&PlayerOptions::default())).unwrap(),
        true,
    );

    assert_eq!(error, None);
    assert_eq!(output, "error: service rshelld is not running");

    let mut world = world;

    prepare_rshell_service(&mut world);

    let (output, error, world) = run_on_host(code, world, true);

    assert_eq!(error, None);
    assert_eq!(
        output,
        "rshell_remove: expected a list of reverse shells or null"
    );

    let (output, error, _) = run_on_host(
        "print include_lib(\"/lib/metaxploit.so\").rshell_remove",
        world,
        true,
    );

    assert_eq!(error, None);
    assert_eq!(
        output,
        "error: rshell portforward is not configured correctly"
    );
}

#[test]
fn should_refuse_reverse_shell_removal_without_internet_or_on_a_ctf_network() {
    let mut world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let (player, router) = prepare_rshell(&mut world);

    block_on(
        world
            .modules
            .core
            .computers
            .processes
            .reboot(router.computer(), false),
    )
    .unwrap();

    let code = "print include_lib(\"/lib/metaxploit.so\").rshell_remove(1)";
    let (output, error, world) = run_on_host(code, world, true);

    assert_eq!(error, None);
    assert_eq!(output, "rshell_remove: No internet connection");

    let network = read_computer(&world, player).network;

    block_on(
        world
            .modules
            .core
            .networks
            .settings
            .set_network_kind(network, NetworkKind::Ctf),
    )
    .unwrap();
    block_on(world.tick(10 * 60)).unwrap();

    let (output, error, _) = run_on_host(code, world, true);

    assert_eq!(error, None);
    assert_eq!(
        output,
        "rshell_remove: The service cannot be started on this network."
    );
}

#[tokio::test]
async fn should_prune_reverse_shells_for_missing_computers_processes_and_disabled_networks() {
    for condition in ["computer", "process", "network"] {
        let mut world = World::generate(&world_options(9, 1)).await.unwrap();
        let (player, router) = prepare_rshell(&mut world);
        let remote = first_remote_service(&world).computer;
        let pid = world
            .modules
            .core
            .computers
            .processes
            .spawn_process(remote, "test", "client", None)
            .await
            .unwrap();

        world
            .modules
            .libraries
            .metaxploit
            .insert_reverse_shell(
                router,
                &ReverseShell {
                    computer: remote,
                    port: 1222,
                    user: "test".to_owned(),
                    pid,
                    creation_second: world.handle.time_seconds(),
                },
            )
            .await
            .unwrap();

        match condition {
            "computer" => world
                .modules
                .core
                .computers
                .remove_computer(remote)
                .await
                .unwrap(),
            "process" => {
                world
                    .modules
                    .core
                    .computers
                    .processes
                    .end_process(remote, pid)
                    .await
                    .unwrap();
            }
            "network" => {
                let address = first_remote_service(&world).address;

                world
                    .modules
                    .core
                    .networks
                    .settings
                    .disable_network(&address, true)
                    .await
                    .unwrap();
            }
            _ => unreachable!(),
        }

        let clients = world
            .modules
            .libraries
            .metaxploit
            .rshell_clients(player)
            .await
            .unwrap()
            .unwrap()
            .unwrap();

        assert!(clients.is_empty(), "{condition}");
        assert!(
            world
                .modules
                .libraries
                .metaxploit
                .reverse_shells(router)
                .await
                .unwrap()
                .is_empty(),
            "{condition}"
        );
    }
}
