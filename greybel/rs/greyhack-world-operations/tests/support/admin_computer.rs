use std::rc::Rc;

use greyhack_behaviour::{LibraryKind, ServiceId};
use greyhack_world::{Library, PlayerOptions, World};
use greyhack_world_operations::WorldServer;
use scheduler::ManualClock;

pub async fn player_server() -> (WorldServer, String, String, String) {
    let mut world = World::new(&PlayerOptions::default()).await.unwrap();
    let player = world.player();
    let file = world
        .file(player, "/lib/libssh.so", false)
        .await
        .unwrap()
        .unwrap();

    world
        .set_library_of(
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
        .install_service(player, ServiceId::Ssh, "1.0.0")
        .await
        .unwrap();

    let public_ip = world.public_ip(player).await.unwrap();
    let lan_ip = world.computer(player).await.unwrap().unwrap().local_ip;
    let password = world.users(player).await.unwrap()[0].password.clone();

    (
        WorldServer::new(world, Rc::new(ManualClock::constant())),
        public_ip,
        lan_ip,
        password,
    )
}
