//! Worlds with networks to connect to by Wi-Fi, and a player computer with saved networks.

use greyhack_world::{
    ComputerId, DeviceLayout, Hardware, MiningHardware, NetCard, NetworkOptions, PlayerOptions,
    RouterId, SavedNetwork, World,
};

use super::layout::{layout, startup_files};

pub const LAN_BSSID: &str = "AA:BB:CC:DD:EE:FF";
pub const MAIN_IP: &str = "45.45.45.45";

/// The player's world with two networks within Wi-Fi range of its own router.
pub async fn world_with_networks() -> (World, RouterId, RouterId) {
    let mut world = World::new(&PlayerOptions::default()).await.unwrap();
    let player = world.player();
    let mut home = world.router_of(player).await.unwrap().unwrap();

    home.map = [0.0, 0.0];
    world.save_router(&home).await.unwrap();

    let network = |public_ip: &str, essid: &str, password: &str, x: f32| NetworkOptions {
        public_ip: Some(public_ip.to_owned()),
        essid: Some(essid.to_owned()),
        wifi_password: Some(password.to_owned()),
        position: Some([x, 0.0]),
        ..NetworkOptions::default()
    };
    let alpha = world
        .add_network(&network("8.8.8.8", "alpha", "pw-a", 0.1))
        .await
        .unwrap();
    let bravo = world
        .add_network(&network("9.9.9.9", "bravo", "pw-b", 0.2))
        .await
        .unwrap();

    (world, alpha, bravo)
}

/// The game's test network, where the LAN router 172.16.3.1 has Wi-Fi, and the player has a Wi-Fi card connected to
/// the main router, saving `saved` (the LAN router with `password` unless it says otherwise). `ethernet` makes the
/// Ethernet card the active one instead.
pub async fn world_on_lan(saved: Vec<SavedNetwork>, ethernet: bool) -> (World, ComputerId) {
    let mut network = layout();

    if let DeviceLayout::Router { draft, .. } = &mut network.nodes[2].device {
        draft.essid = Some("lan".to_owned());
        draft.bssid = Some(LAN_BSSID.to_owned());
        draft.wifi_password = "lan-pw".to_owned();
    }

    if let DeviceLayout::Computer(player) = &mut network.nodes[1].device {
        let cards = vec![
            NetCard {
                active: !ethernet,
                ..NetCard::new(true, false)
            },
            NetCard {
                active: ethernet,
                ..NetCard::new(false, false)
            },
        ];

        player.hardware = Hardware::standard(2048, 10_000.0, &MiningHardware::default(), cards);
        player.saved_networks = saved;
        startup_files(player);
    }

    let world = World::new(&PlayerOptions {
        network: Some(network),
        ..PlayerOptions::default()
    })
    .await
    .unwrap();
    let player = world.player();

    (world, player)
}

pub fn lan_network(password: &str) -> SavedNetwork {
    SavedNetwork {
        bssid: LAN_BSSID.to_owned(),
        essid: "lan".to_owned(),
        password: password.to_owned(),
        public_ip: MAIN_IP.to_owned(),
    }
}

pub async fn gateway(world: &World, computer: ComputerId) -> String {
    world.computer(computer).await.unwrap().unwrap().gateway
}
