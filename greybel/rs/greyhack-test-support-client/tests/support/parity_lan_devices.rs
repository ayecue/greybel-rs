//! What the LAN device parity test reads of generated networks in the game and in Greybel.

use greybel_test_support::future::block_on;
use greyhack_behaviour::{DeviceKind, NetworkKind, WorldSeed, kind_for_ip};
use greyhack_test_support_client::TestClient;
use greyhack_test_support_client::protocol::test_support::{LanNodeKind, NetworkSnapshot};
use greyhack_world::{PlayerOptions, World};
use greyhack_world_creation::NetworkOptions;
use greyhack_world_missions::request_for;

use super::game::join_network;

/// Networks generated for each kind of network.
pub const NETWORKS: usize = 6;

/// The game's network kinds and the Greybel kinds they are.
pub const KINDS: [(&str, NetworkKind); 5] = [
    ("Supermercados", NetworkKind::Supermarket),
    ("Bancos", NetworkKind::Bank),
    ("FastFood", NetworkKind::FastFood),
    ("Comisaria", NetworkKind::Police),
    ("Particulares", NetworkKind::Residential),
];

/// The cameras, fridges and microwaves of one network.
#[derive(Debug, Default, Clone, Copy)]
pub struct Gadgets {
    cameras: u32,
    fridges: u32,
    microwaves: u32,
}

impl Gadgets {
    pub fn of<'a>(kinds: impl Iterator<Item = &'a str>) -> Self {
        let mut gadgets = Self::default();

        for kind in kinds {
            match kind {
                "CAMERA" => gadgets.cameras += 1,
                "FRIDGE" => gadgets.fridges += 1,
                "MICROWAVE" => gadgets.microwaves += 1,
                _ => {}
            }
        }

        gadgets
    }

    pub fn any(&self) -> bool {
        self.cameras + self.fridges + self.microwaves > 0
    }
}

/// What the rule fixes for the networks of one kind (`GetRandomTypeDevice`, `NetworkLan.PostGeneration`).
#[derive(Debug, PartialEq, Eq)]
pub struct Rules {
    /// No network has more than six cameras.
    cameras_within_limit: bool,
    /// No network has a second fridge or a second microwave.
    appliances_within_limit: bool,
    /// Every network with a fridge or a microwave has a camera.
    appliances_have_a_camera: bool,
    /// No network has a camera, a fridge or a microwave.
    no_gadgets: bool,
}

impl Rules {
    pub fn of(networks: &[Gadgets]) -> Self {
        Self {
            cameras_within_limit: networks.iter().all(|network| network.cameras <= 6),
            appliances_within_limit: networks
                .iter()
                .all(|network| network.fridges <= 1 && network.microwaves <= 1),
            appliances_have_a_camera: networks
                .iter()
                .all(|network| network.fridges + network.microwaves == 0 || network.cameras > 0),
            no_gadgets: networks.iter().all(|network| !network.any()),
        }
    }
}

/// The gadgets of the networks the game spawns for `kind`, each read after the player joins it. A network whose mission
/// gives it another kind is left out: it is generated as that kind (`NetworkLan.NetworkLan` sets the router's kind
/// before `NetworkLan.Configure` builds the LAN). The snapshot can't tell: a router read back from the database takes
/// its node's kind again (`Database.RestoreRouter`), so the mission is looked up from the game's world secret.
pub async fn game_networks(client: &TestClient, kind: &str) -> Vec<Gadgets> {
    let mut networks = Vec::new();

    for _ in 0..NETWORKS {
        let network = client
            .spawn_network(kind)
            .await
            .expect("a network is generated");

        join_network(client, &network.public_ip).await;

        let snapshot = client.network_state().await.expect("the network is read");
        let seed = WorldSeed::new(snapshot.world_secret.to_vec());

        if snapshot.kind == kind && !mission_changes_the_kind(&seed, &network.public_ip) {
            networks.push(game_gadgets(&snapshot));
        }
    }

    networks
}

pub fn game_gadgets(snapshot: &NetworkSnapshot) -> Gadgets {
    Gadgets::of(snapshot.nodes.iter().filter_map(|node| match &node.node {
        LanNodeKind::Computer(computer) => Some(computer.kind.as_ref()),
        _ => None,
    }))
}

/// Whether the mission of the network at `address` gives it another kind, whether or not a zero-day mission is open
/// (`NetworkLan.NetworkLan`, `ZeroDaySystem.IsRumorTime`).
fn mission_changes_the_kind(seed: &WorldSeed, address: &str) -> bool {
    [false, true].into_iter().any(|open| {
        request_for(seed, address, open)
            .and_then(|request| request.network_kind())
            .is_some()
    })
}

/// The gadgets of the networks Greybel generates for `kind`, without those a mission gives another kind.
pub fn greybel_networks(kind: NetworkKind) -> Vec<Gadgets> {
    std::thread::scope(|scope| {
        scope
            .spawn(|| {
                block_on(async {
                    let world = World::new(&PlayerOptions::default())
                        .await
                        .expect("a world");
                    let addresses: Vec<String> = (1..=u16::MAX)
                        .map(|n| format!("8.8.{}.{}", n >> 8, n & 0xff))
                        .filter(|address| kind_for_ip(&world.handle.world_seed(), address) == kind)
                        .filter(|address| {
                            !mission_changes_the_kind(&world.handle.world_seed(), address)
                        })
                        .take(NETWORKS)
                        .collect();
                    let mut networks = Vec::new();

                    for address in addresses {
                        world
                            .modules
                            .creation
                            .network_generation
                            .add_network(&NetworkOptions {
                                public_ip: Some(address.clone()),
                                ..NetworkOptions::default()
                            })
                            .await
                            .expect("a network is generated");

                        let router = world
                            .modules
                            .core
                            .networks
                            .routers
                            .router_at(&address)
                            .await
                            .expect("the router is read")
                            .expect("the network has a router");
                        let lan = world
                            .modules
                            .core
                            .networks
                            .lan
                            .lan(router.network)
                            .await
                            .expect("the LAN is read")
                            .expect("the network has a LAN");

                        networks.push(Gadgets::of(lan.nodes().filter_map(
                            |(_, node)| match node.kind {
                                DeviceKind::Camera => Some("CAMERA"),
                                DeviceKind::Fridge => Some("FRIDGE"),
                                DeviceKind::Microwave => Some("MICROWAVE"),
                                _ => None,
                            },
                        )));
                    }

                    networks
                })
            })
            .join()
            .expect("the networks are generated")
    })
}
