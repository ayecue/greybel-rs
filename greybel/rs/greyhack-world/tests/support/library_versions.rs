//! Helpers of the library version tests.

use greyhack_behaviour::{LibraryKind, MAX_WEAKNESS_USES, Reward};
use greyhack_world::{PlayerOptions, World};
use greyhack_world_libraries::Library;

use super::exploit_shop::{VERSION, install_library, named_weakness, zone};

pub const SSH: LibraryKind = LibraryKind::Ssh;

pub async fn newest(world: &World, kind: LibraryKind) -> Library {
    world
        .modules
        .libraries
        .generator
        .latest_full_library(kind)
        .await
        .unwrap()
        .expect("the world has the library")
}

pub async fn use_up(world: &mut World, unsec_value: &str) {
    for _ in 0..MAX_WEAKNESS_USES {
        world
            .modules
            .libraries
            .versions
            .count_weakness_use(SSH, "0x10", unsec_value)
            .await
            .unwrap();
    }
}

/// A world whose newest libssh.so has a visible zone with two weaknesses and a hidden zone with one.
pub async fn hidden_world() -> World {
    let world = World::new(&PlayerOptions::default()).await.unwrap();

    install_library(
        &world,
        world.handle.player(),
        SSH,
        VERSION,
        vec![
            zone(
                "0x10",
                false,
                vec![
                    named_weakness("buffer", Reward::Shell, "guest", true),
                    named_weakness("loop", Reward::Computer, "root", true),
                ],
            ),
            zone(
                "0x20",
                true,
                vec![named_weakness("deep", Reward::Shell, "root", true)],
            ),
        ],
    )
    .await;

    world
}
