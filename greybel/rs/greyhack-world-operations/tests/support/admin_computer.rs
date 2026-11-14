use std::rc::Rc;

use greyhack_behaviour::{LibraryKind, ServiceId};
use greyhack_world::{Library, PlayerOptions, World};
use greyhack_world_operations::WorldServer;
use scheduler::ManualClock;

pub async fn player_server() -> (WorldServer, String, String, String) {
    let world = World::new(&PlayerOptions::default()).await.unwrap();
    let player = world.handle.player();
    let file = world
        .modules
        .core
        .files
        .lookup
        .file(player, "/lib/libssh.so", false)
        .await
        .unwrap()
        .unwrap();

    world
        .modules
        .libraries
        .generator
        .set_full_library_of(
            &file,
            Some(&Library {
                kind: LibraryKind::Ssh,
                version: "1.0.0".to_owned(),
                zones: Vec::new(),
            }),
        )
        .await
        .unwrap();
    world
        .modules
        .services
        .installs
        .install_service(player, ServiceId::Ssh, "1.0.0")
        .await
        .unwrap();

    let public_ip = world
        .modules
        .core
        .networks
        .topology
        .public_ip(player)
        .await
        .unwrap();
    let lan_ip = world
        .modules
        .core
        .networks
        .topology
        .computer(player)
        .await
        .unwrap()
        .unwrap()
        .local_ip;
    let password = world
        .modules
        .core
        .computers
        .users
        .users(player)
        .await
        .unwrap()[0]
        .password
        .clone();

    (
        WorldServer::new(world, Rc::new(ManualClock::constant()))
            .await
            .unwrap(),
        public_ip,
        lan_ip,
        password,
    )
}
