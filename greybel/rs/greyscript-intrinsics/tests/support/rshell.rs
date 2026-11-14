use greybel_test_support::future::block_on;
use greyhack_behaviour::{FileData, LibraryKind, ServiceId};
use greyhack_world::{ComputerId, Library, RouterId, World};

use super::{add_library, player_router};

pub fn prepare_rshell(world: &mut World) -> (ComputerId, RouterId) {
    let player = world.handle.player();
    let router = player_router(world);

    block_on(
        world
            .modules
            .services
            .installs
            .install_service(player, ServiceId::Rshell, "1.0.0"),
    )
    .unwrap();
    add_library(
        world,
        player,
        "librshell.so",
        FileData::library(1000),
        Some(Library {
            kind: LibraryKind::Rshell,
            version: "1.0.0".to_owned(),
            zones: Vec::new(),
        }),
    );
    assert!(
        block_on(
            world
                .modules
                .core
                .networks
                .router_config
                .forward_port(router, 1222, player, 1222)
        )
        .expect("the world writes")
    );

    (player, router)
}
