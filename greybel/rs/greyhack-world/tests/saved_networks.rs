use greyhack_behaviour::SAFE_MODE_TERMINAL;
use greyhack_world::{PlayerOptions, World};
use greyhack_world_core::{
    ComputerFailure, NetCard, NetCardFailure, SavedNetwork, SavedWifiFailure, SwitchResult,
};
use greyhack_world_creation::DeviceLayout;

use crate::support::layout::layout;
use crate::support::saved_networks::{
    MAIN_IP, gateway, lan_network, world_on_lan, world_with_networks,
};

/// Player Ethernet checks the rental home and its ISP before changing the LAN (`NetworkLan.ConnectEthernet`).
#[tokio::test]
async fn should_validate_the_player_rental_home_for_ethernet() {
    let mut layout = layout();

    for node in &mut layout.nodes {
        if let DeviceLayout::Computer(computer) = &mut node.device
            && computer.is_player
        {
            computer.hardware.net_cards.push(NetCard::new(false, false));
        }
    }

    let world = World::new(&PlayerOptions {
        network: Some(layout),
        ..PlayerOptions::default()
    })
    .await
    .unwrap();
    let player = world.handle.player();
    let connect = || {
        world.modules.core.networks.net_cards.join_ethernet(
            player,
            "root",
            ("eth0", "192.168.1.250", "192.168.1.1"),
            false,
        )
    };
    let result = connect().await.unwrap();

    assert!(matches!(
        result.result,
        SwitchResult::Refusal(ComputerFailure::NetCard(NetCardFailure::NoRentalHome))
    ));

    let mut computer = greybel_test_support::world::computer(&world, player).await;

    computer.rental_home_ip = "203.0.113.1".to_owned();

    world
        .modules
        .core
        .networks
        .topology
        .save_computer(&computer)
        .await
        .unwrap();

    let result = connect().await.unwrap();

    assert!(matches!(
        result.result,
        SwitchResult::Refusal(ComputerFailure::NetCard(NetCardFailure::MissingIsp))
    ));
}

/// Connecting by Wi-Fi remembers the router as the computer's most recent network (`ConfigOS.SetIpAddress`).
#[tokio::test]
async fn should_save_the_network_it_connects_to_by_wifi() {
    let (world, alpha, _) = world_with_networks().await;
    let player = world.handle.player();

    assert_eq!(
        world
            .modules
            .core
            .networks
            .net_cards
            .saved_networks(player)
            .await
            .unwrap(),
        []
    );

    world
        .modules
        .core
        .networks
        .net_cards
        .connect_wifi(player, alpha)
        .await
        .unwrap()
        .unwrap();

    let router = world
        .modules
        .core
        .networks
        .routers
        .router(alpha)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        world
            .modules
            .core
            .networks
            .net_cards
            .saved_networks(player)
            .await
            .unwrap(),
        [SavedNetwork::of(&router)]
    );
    assert_eq!(
        world
            .modules
            .core
            .networks
            .net_cards
            .saved_networks(player)
            .await
            .unwrap()[0],
        SavedNetwork {
            bssid: router.bssid.clone().unwrap(),
            essid: "alpha".to_owned(),
            password: "pw-a".to_owned(),
            public_ip: "8.8.8.8".to_owned(),
        }
    );
}

/// Connecting again to a saved network removes the old entry and adds it last (`ConfigOS.AddSavedNetwork`).
#[tokio::test]
async fn should_move_a_network_connected_again_to_the_end() {
    let (world, alpha, bravo) = world_with_networks().await;
    let player = world.handle.player();

    world
        .modules
        .core
        .networks
        .net_cards
        .connect_wifi(player, alpha)
        .await
        .unwrap()
        .unwrap();
    world
        .modules
        .core
        .networks
        .net_cards
        .connect_wifi(player, bravo)
        .await
        .unwrap()
        .unwrap();

    let order = |saved: Vec<SavedNetwork>| {
        saved
            .into_iter()
            .map(|network| network.essid)
            .collect::<Vec<_>>()
    };

    assert_eq!(
        order(
            world
                .modules
                .core
                .networks
                .net_cards
                .saved_networks(player)
                .await
                .unwrap()
        ),
        ["alpha", "bravo"]
    );

    world
        .modules
        .core
        .networks
        .net_cards
        .connect_wifi(player, alpha)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        order(
            world
                .modules
                .core
                .networks
                .net_cards
                .saved_networks(player)
                .await
                .unwrap()
        ),
        ["bravo", "alpha"]
    );
}

/// The password saved is the router's at the time, and a network connected again saves the current one.
#[tokio::test]
async fn should_save_the_password_the_router_had_when_connected() {
    let (world, alpha, _) = world_with_networks().await;
    let player = world.handle.player();

    world
        .modules
        .core
        .networks
        .net_cards
        .connect_wifi(player, alpha)
        .await
        .unwrap()
        .unwrap();

    let mut router = world
        .modules
        .core
        .networks
        .routers
        .router(alpha)
        .await
        .unwrap()
        .unwrap();

    router.wifi_password = "changed".to_owned();
    world
        .modules
        .core
        .networks
        .routers
        .save_router(&router)
        .await
        .unwrap();

    assert_eq!(
        world
            .modules
            .core
            .networks
            .net_cards
            .saved_networks(player)
            .await
            .unwrap()[0]
            .password,
        "pw-a"
    );

    world
        .modules
        .core
        .networks
        .net_cards
        .connect_wifi(player, alpha)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        world
            .modules
            .core
            .networks
            .net_cards
            .saved_networks(player)
            .await
            .unwrap(),
        [SavedNetwork::of(&router)]
    );
}

/// Starting the player's computer connects it to the most recent saved network again
/// (`PlayerComputer.AutoConnectWifi`), here found through the LAN router of its network.
#[tokio::test]
async fn should_connect_to_the_most_recent_saved_network_when_the_player_restarts() {
    let (world, player) = world_on_lan(vec![lan_network("lan-pw")], false).await;

    assert_eq!(gateway(&world, player).await, "192.168.1.1");

    assert!(
        world
            .modules
            .core
            .computers
            .processes
            .reboot_player(false)
            .await
            .unwrap()
    );

    assert_eq!(gateway(&world, player).await, "172.16.3.1");
    assert_eq!(
        world
            .modules
            .core
            .networks
            .net_cards
            .saved_networks(player)
            .await
            .unwrap(),
        [lan_network("lan-pw")]
    );
    assert!(
        !world
            .modules
            .core
            .computers
            .processes
            .in_safe_mode(player)
            .await
            .unwrap()
    );
}

/// Only the most recent network counts, whatever came before.
#[tokio::test]
async fn should_ignore_older_saved_networks_when_the_player_restarts() {
    let older = SavedNetwork {
        bssid: "00:00:00:00:00:00".to_owned(),
        essid: "missing".to_owned(),
        password: "x".to_owned(),
        public_ip: "1.2.3.4".to_owned(),
    };
    let (world, player) = world_on_lan(vec![lan_network("lan-pw"), older], false).await;

    world
        .modules
        .core
        .computers
        .processes
        .reboot_player(false)
        .await
        .unwrap();

    assert_eq!(gateway(&world, player).await, "192.168.1.1");
}

/// The desktop doesn't reconnect a computer whose Ethernet card is the active one.
#[tokio::test]
async fn should_not_connect_by_wifi_when_ethernet_is_active() {
    let (world, player) = world_on_lan(vec![lan_network("lan-pw")], true).await;

    world
        .modules
        .core
        .computers
        .processes
        .reboot_player(false)
        .await
        .unwrap();

    assert_eq!(gateway(&world, player).await, "192.168.1.1");
}

/// Safe mode starts the protected terminal instead of the desktop, which is what connects the computer.
#[tokio::test]
async fn should_not_connect_by_wifi_in_safe_mode_and_start_the_safe_terminal() {
    let (world, player) = world_on_lan(vec![lan_network("lan-pw")], false).await;

    assert!(
        world
            .modules
            .core
            .computers
            .processes
            .reboot_player(true)
            .await
            .unwrap()
    );

    assert_eq!(gateway(&world, player).await, "192.168.1.1");
    assert!(
        world
            .modules
            .core
            .computers
            .processes
            .in_safe_mode(player)
            .await
            .unwrap()
    );

    let terminal = world
        .modules
        .core
        .computers
        .processes
        .processes(player)
        .await
        .unwrap()
        .into_iter()
        .find(|process| process.name == SAFE_MODE_TERMINAL)
        .expect("the safe-mode terminal");

    assert_eq!(
        (
            terminal.user.as_str(),
            terminal.cpu,
            terminal.ram_mb,
            terminal.protected,
            terminal.is_terminal,
        ),
        ("root", 0.0, 0.1, true, true)
    );
}

/// The next restart leaves safe mode: its terminal is not one of the processes the player keeps
/// (`PlayerComputer.RemoveAllProcess`), and the desktop starts again.
#[tokio::test]
async fn should_leave_safe_mode_on_the_next_normal_restart() {
    let (world, player) = world_on_lan(vec![lan_network("lan-pw")], false).await;

    world
        .modules
        .core
        .computers
        .processes
        .reboot_player(true)
        .await
        .unwrap();
    world
        .modules
        .core
        .computers
        .processes
        .reboot_player(false)
        .await
        .unwrap();

    assert!(
        !world
            .modules
            .core
            .computers
            .processes
            .in_safe_mode(player)
            .await
            .unwrap()
    );
    assert_eq!(gateway(&world, player).await, "172.16.3.1");
}

/// What leaves the computer where it is when the saved network can't be joined
/// (`NetworkHelperServer.IwconfigUIServerRpc`), in the game's order.
#[tokio::test]
async fn should_refuse_a_saved_network_in_the_games_order() {
    let (world, player) = world_on_lan(vec![lan_network("other")], false).await;

    assert_eq!(
        world
            .modules
            .core
            .networks
            .net_cards
            .auto_connect_wifi()
            .await
            .unwrap(),
        Err(SavedWifiFailure::IncorrectPassword)
    );

    world
        .modules
        .core
        .networks
        .settings
        .disable_network(MAIN_IP, true)
        .await
        .unwrap();

    assert_eq!(
        world
            .modules
            .core
            .networks
            .net_cards
            .auto_connect_wifi()
            .await
            .unwrap(),
        Err(SavedWifiFailure::NonPayment)
    );
    assert_eq!(gateway(&world, player).await, "192.168.1.1");

    // The LAN router isn't a main router: it is found through the public address it was saved with.
    let (world, _) = world_on_lan(
        vec![SavedNetwork {
            public_ip: "1.2.3.4".to_owned(),
            ..lan_network("lan-pw")
        }],
        false,
    )
    .await;

    assert_eq!(
        world
            .modules
            .core
            .networks
            .net_cards
            .auto_connect_wifi()
            .await
            .unwrap(),
        Err(SavedWifiFailure::UnknownRouter)
    );

    let (world, _) = world_on_lan(
        vec![SavedNetwork {
            bssid: "11:11:11:11:11:11".to_owned(),
            public_ip: "1.2.3.4".to_owned(),
            ..lan_network("lan-pw")
        }],
        false,
    )
    .await;

    assert_eq!(
        world
            .modules
            .core
            .networks
            .net_cards
            .auto_connect_wifi()
            .await
            .unwrap(),
        Err(SavedWifiFailure::UnknownRouter)
    );
}

/// Without a saved network the game only marks Wi-Fi active.
#[tokio::test]
async fn should_only_enable_wifi_without_a_saved_network() {
    let (world, player) = world_on_lan(Vec::new(), true).await;

    assert_eq!(
        world
            .modules
            .core
            .networks
            .net_cards
            .auto_connect_wifi()
            .await
            .unwrap(),
        Ok(())
    );
    assert!(
        world
            .modules
            .core
            .networks
            .net_cards
            .active_net_card(player)
            .await
            .unwrap()
            .unwrap()
            .wifi
    );
    assert_eq!(gateway(&world, player).await, "192.168.1.1");
}

/// A computer without a `wlan0` card can't connect to what it saved.
#[tokio::test]
async fn should_refuse_a_saved_network_without_a_wlan0_card() {
    let mut network = layout();

    if let DeviceLayout::Computer(player) = &mut network.nodes[1].device {
        player.saved_networks = vec![lan_network("lan-pw")];
    }

    let world = World::new(&PlayerOptions {
        network: Some(network),
        ..PlayerOptions::default()
    })
    .await
    .unwrap();

    assert_eq!(
        world
            .modules
            .core
            .networks
            .net_cards
            .auto_connect_wifi()
            .await
            .unwrap(),
        Err(SavedWifiFailure::NoWlan0)
    );
}

/// A computer that fails its startup checks starts the safe-mode terminal as well (`Computer.CheckInicio`).
#[tokio::test]
async fn should_start_the_safe_terminal_when_the_startup_checks_fail() {
    let (world, player) = world_on_lan(vec![lan_network("lan-pw")], false).await;

    assert!(
        world
            .modules
            .core
            .computers
            .startup
            .startup_failures(player)
            .await
            .unwrap()
            .is_empty()
    );

    let kernel = world
        .modules
        .core
        .files
        .lookup
        .file(player, "/boot/System.map", false)
        .await
        .unwrap()
        .expect("a startup file");

    world
        .modules
        .core
        .files
        .changes
        .remove_node(&kernel)
        .await
        .unwrap();
    world
        .modules
        .core
        .computers
        .processes
        .reboot_player(false)
        .await
        .unwrap();

    assert!(
        world
            .modules
            .core
            .computers
            .processes
            .in_safe_mode(player)
            .await
            .unwrap()
    );
    assert_eq!(gateway(&world, player).await, "192.168.1.1");
}
