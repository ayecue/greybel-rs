//! The helpers of greyscript-intrinsics's integration tests.

#![allow(dead_code)]

pub mod apt_repository;
pub mod bank_payments;
pub mod ctf_scoring;
pub mod debug;
pub mod economy;
pub mod exploit_shop;
pub mod filesystem;
pub mod game;
pub mod guilds;
pub mod mail;
pub mod network;
pub mod processes;
pub mod regex;
pub mod rshell;
pub mod safe_mode;
pub mod saved_networks;
pub mod scan;
pub mod scripts;
pub mod semantics;
pub mod smart;
pub mod sniffer;
pub mod system_log;
pub mod throttle;
pub mod traffic;

use greybel_test_support::future::block_on;
use greybel_test_support::world;
pub use greybel_test_support::world::{save_copy, world_options};
use greyhack_behaviour::{FileData, PermissionSet};
use greyhack_world::{Computer, ComputerId, Library, NodeDraft, Router, RouterId, World};

/// The computer `id` of `world`, as it is now.
pub fn read_computer(world: &World, id: ComputerId) -> Computer {
    block_on(world::computer(world, id))
}

/// The router `id` of `world`, as it is now.
pub fn read_router(world: &World, id: RouterId) -> Router {
    block_on(world::router(world, id))
}

/// Every router of `world`, by id.
pub fn read_routers(world: &World) -> Vec<Router> {
    block_on(world::routers(world))
}

/// A copy of `world`, loaded from a new database.
pub fn copy(world: &mut World) -> World {
    let database = block_on(save_copy(world));

    block_on(World::load(&database)).expect("the world loads")
}

/// The router of the player's network in `world`.
pub fn player_router(world: &World) -> RouterId {
    read_computer(world, world.player())
        .router
        .expect("the player's network")
}

/// Runs `change` on the computer `id` of `world`.
pub fn change_computer<T>(
    world: &mut World,
    id: ComputerId,
    change: impl FnOnce(&mut Computer) -> T,
) -> T {
    block_on(world::change_computer(world, id, change))
}

/// Runs `change` on the router `id` of `world`.
pub fn change_router<T>(
    world: &mut World,
    id: RouterId,
    change: impl FnOnce(&mut Router) -> T,
) -> T {
    block_on(world::change_router(world, id, change))
}

/// The library of the library file at `path` on `computer` of `world`.
pub fn library(world: &World, computer: ComputerId, path: &str) -> Library {
    block_on(world::library(world, computer, path))
}

/// Adds the library file `name` with `data` and `library` to `/lib` of `computer`, owned by root.
pub fn add_library(
    world: &mut World,
    computer: ComputerId,
    name: &str,
    data: FileData,
    library: Option<Library>,
) {
    block_on(async {
        let folder = world
            .folder(computer, "/lib", true)
            .await
            .expect("the world reads")
            .expect("/lib");
        let mut node = NodeDraft::file(name, "root", PermissionSet::default_for(false), data);

        node.library = library.map(Box::new);
        world
            .add_node(&folder, &node)
            .await
            .expect("the world writes");
    });
}

/// Sets the library of the file at `path` on `computer`.
pub fn set_library(world: &World, computer: ComputerId, path: &str, library: Library) {
    block_on(async {
        let file = world
            .file(computer, path, true)
            .await
            .expect("the world reads")
            .expect("the file");

        world
            .set_library_of(&file, Some(&library))
            .await
            .expect("the world writes");
    });
}

/// Runs `change` on the library of the library file at `path` on `computer` of `world`.
pub fn change_library<T>(
    world: &mut World,
    computer: ComputerId,
    path: &str,
    change: impl FnOnce(&mut Library) -> T,
) -> T {
    block_on(async {
        let file = world
            .file(computer, path, true)
            .await
            .expect("the world reads")
            .expect("the file");
        let mut library = world
            .library_of(&file)
            .await
            .expect("the world reads")
            .expect("a library");
        let result = change(&mut library);

        world
            .set_library_of(&file, Some(&library))
            .await
            .expect("the world writes");

        result
    })
}
