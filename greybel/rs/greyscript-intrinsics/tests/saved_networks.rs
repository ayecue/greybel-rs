use greybel_test_support::future::block_on;

use crate::support::saved_networks::{connect, run, world_with_networks};
use crate::support::{player_router, read_computer, read_router};

/// A `connect_wifi` that goes through saves its network (`ConfigOS.AddSavedNetwork`), the most recent last, and one
/// that is refused doesn't.
#[test]
fn should_save_the_networks_the_script_connects_to_most_recent_last() {
    let (world, alpha, bravo) = world_with_networks();
    let code = format!(
        "c = get_shell.host_computer\n{}\nwait(1.1)\n{}\nwait(1.1)\n{}\nwait(1.1)\n{}",
        connect(&world, alpha, "pw-a"),
        connect(&world, bravo, "wrong"),
        connect(&world, bravo, "pw-b"),
        connect(&world, alpha, "pw-a"),
    );

    let (output, error, world) = run(&code, world);

    assert_eq!(error, None);
    assert_eq!(output, "1\nCan't connect. Incorrect password.\n1\n1");

    let saved: Vec<_> = block_on(
        world
            .modules
            .core
            .networks
            .net_cards
            .saved_networks(world.handle.player()),
    )
    .unwrap()
    .into_iter()
    .map(|network| (network.essid, network.password, network.public_ip))
    .collect();

    assert_eq!(
        saved,
        [
            ("bravo".to_owned(), "pw-b".to_owned(), "9.9.9.9".to_owned()),
            ("alpha".to_owned(), "pw-a".to_owned(), "8.8.8.8".to_owned()),
        ]
    );
}

/// Restarting the player's computer connects it to the most recent saved network again: the one connected to last,
/// not the one connected to first or the one before it.
#[test]
fn should_stay_on_the_most_recent_network_after_the_player_restarts() {
    let (world, alpha, bravo) = world_with_networks();
    let code = format!(
        "c = get_shell.host_computer\n{}\nwait(1.1)\n{}\nwait(1.1)\n{}\nprint c.reboot",
        connect(&world, alpha, "pw-a"),
        connect(&world, bravo, "pw-b"),
        connect(&world, alpha, "pw-a"),
    );

    let (output, error, world) = run(&code, world);

    assert_eq!(error, None);
    assert_eq!(output, "1\n1\n1\n1");
    assert_eq!(player_router(&world), alpha);
    assert_eq!(
        read_computer(&world, world.handle.player()).gateway,
        read_router(&world, alpha).local_ip
    );
}

/// A restart in safe mode doesn't start the desktop, which is what connects the computer.
#[test]
fn should_not_reconnect_the_player_in_safe_mode() {
    let (world, alpha, _) = world_with_networks();
    let code = format!(
        "c = get_shell.host_computer\n{}\nprint c.reboot(1)",
        connect(&world, alpha, "pw-a")
    );

    let (output, error, world) = run(&code, world);

    assert_eq!(error, None);
    assert_eq!(output, "1\n1");
    assert_eq!(player_router(&world), alpha);
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
}
