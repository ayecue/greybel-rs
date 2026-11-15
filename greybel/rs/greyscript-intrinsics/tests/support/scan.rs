use greybel_test_support::future::block_on;
use greyhack_behaviour::{LibraryKind, ServiceId};
use greyhack_world::World;
use greyhack_world_core::ComputerId;
use greyhack_world_libraries::Library;

use super::network::literal;
use super::{library, read_routers, set_library, world_options};

pub struct Target {
    pub address: String,
    pub computer: ComputerId,
}

pub fn ssh_target(world: &World) -> Target {
    read_routers(world)
        .into_iter()
        .skip(1)
        .find_map(|router| {
            block_on(world.modules.core.networks.topology.devices(router.id))
                .expect("the world reads")
                .into_iter()
                .find_map(|id| {
                    block_on(world.modules.core.computers.processes.ports(id))
                        .expect("the world reads")
                        .iter()
                        .any(|port| port.service == ServiceId::Ssh)
                        .then(|| Target {
                            address: router.public_ip.clone(),
                            computer: id,
                        })
                })
        })
        .expect("a remote SSH target")
}

pub fn scan_world(metaxploit_version: &str, library_version: &str) -> (World, Target, String) {
    let world = block_on(World::generate(&world_options(7, 3))).expect("a generated world");
    let target = ssh_target(&world);
    let player = world.handle.player();

    set_library(
        &world,
        player,
        "/lib/metaxploit.so",
        &Library {
            kind: LibraryKind::Metaxploit,
            version: metaxploit_version.to_owned(),
            zones: Vec::new(),
        },
    );

    let source = library(&world, target.computer, "/lib/libssh.so");

    set_library(
        &world,
        player,
        "/lib/libssh.so",
        &Library {
            kind: LibraryKind::Ssh,
            version: library_version.to_owned(),
            zones: source.zones,
        },
    );

    // The library the player has now: one of that version already in the world keeps its own zones.
    let installed = library(&world, player, "/lib/libssh.so");
    let address = installed
        .zones
        .iter()
        .find(|zone| !zone.hide)
        .unwrap()
        .address
        .clone();

    let code = format!(
        "meta = include_lib(\"/lib/metaxploit.so\")\nlib = meta.load(\"/lib/libssh.so\")\nprint meta.scan(lib).len\nprint meta.scan_address(lib, {})",
        literal(&address)
    );

    (world, target, code)
}
