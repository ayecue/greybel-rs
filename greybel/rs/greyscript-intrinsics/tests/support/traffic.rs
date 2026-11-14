use greybel_test_support::future::block_on;
use greyhack_behaviour::{FIRST_VERSION, FileData, LibraryKind, NetworkKind, license_plate};
use greyhack_world::{JobRole, Library, NetworkId, PersonId, PlayerOptions, TrafficNet, World};

use super::{add_library, read_computer};

/// A world whose player is on a police network with `libtrafficnet.so`: the camera system opens with Commissioner
/// Alice's account `alice`/`secret`, and the NPC `Smith` of the network has a car, driving it now.
pub struct TrafficWorld {
    pub world: World,
    pub network: NetworkId,
    pub plate: String,
    pub driver: PersonId,
}

pub fn world_with_traffic() -> TrafficWorld {
    let mut world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let player = world.handle.player();
    let network = read_computer(&world, player).network;
    let public_ip = block_on(world.modules.core.networks.topology.public_ip(player)).unwrap();

    block_on(
        world
            .modules
            .core
            .networks
            .settings
            .set_network_kind(network, NetworkKind::Police),
    )
    .expect("the network kind is saved");
    add_library(
        &mut world,
        player,
        "libtrafficnet.so",
        FileData::library(0),
        Some(Library {
            kind: LibraryKind::TrafficNet,
            version: FIRST_VERSION.to_owned(),
            zones: Vec::new(),
        }),
    );

    let officer = add_npc(&mut world, "alice", "Alice alice");
    let driver = add_npc(&mut world, "Smith", "John Smith");
    let plate = license_plate(&world.handle.world_seed(), &public_ip, "Smith").unwrap();

    block_on(
        world
            .modules
            .npcs
            .jobs
            .set_npc_job(officer, JobRole::Commissioner, None),
    )
    .unwrap();
    block_on(
        world
            .modules
            .npcs
            .jobs
            .set_npc_job(driver, JobRole::Sales, Some(plate.clone())),
    )
    .unwrap();
    block_on(world.modules.smart_devices.traffic.save_traffic(
        network,
        &TrafficNet {
            credentials: Some(officer),
            ..TrafficNet::default()
        },
    ))
    .unwrap();

    // The clock moves on in quarter hours until the driver drives to or from work.
    for _ in 0..7 * 96 {
        if block_on(
            world
                .modules
                .smart_devices
                .traffic
                .locate_vehicle(network, &plate, "secret", false),
        )
        .unwrap()
        .is_ok()
        {
            return TrafficWorld {
                world,
                network,
                plate,
                driver,
            };
        }

        block_on(world.tick(900)).unwrap();
    }

    panic!("the driver drives within a week");
}

/// Adds the player computer's NPC `user`, with the password `secret`.
fn add_npc(world: &mut World, user: &str, name: &str) -> PersonId {
    let player = world.handle.player();

    block_on(
        world
            .modules
            .core
            .computers
            .users
            .add_user(player, user, "secret"),
    )
    .unwrap()
    .unwrap();

    let at = world.handle.time_seconds();

    block_on(world.modules.npcs.npcs.add_npc(player, user, name, at))
        .unwrap()
        .unwrap()
}

/// Enables `network`'s traffic override, as the traffic-lights weakness does.
pub fn enable_override(world: &mut World, network: NetworkId) {
    let mut traffic = block_on(world.modules.smart_devices.traffic.traffic(network)).unwrap();

    traffic.override_enabled = true;
    block_on(
        world
            .modules
            .smart_devices
            .traffic
            .save_traffic(network, &traffic),
    )
    .unwrap();
}
