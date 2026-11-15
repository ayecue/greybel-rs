use greyhack_behaviour::{FileData, PermissionSet, ROOT_USER};
use greyhack_world::{PlayerOptions, World};
use greyhack_world_core::{ComputerId, NodeDraft};

use crate::support::factory_filesystem::{npc, world as generated_world};

/// Builds a small tree in a real world's filesystem, including a symlink to the user's folder.
pub async fn tree() -> (World, ComputerId) {
    let world = World::new(&PlayerOptions::default())
        .await
        .expect("the world generates");
    let computer = world.handle.player();
    let root = world
        .modules
        .core
        .files
        .lookup
        .root(computer)
        .await
        .unwrap()
        .expect("a root folder");
    let home = world
        .modules
        .core
        .files
        .lookup
        .folder(computer, "/home", true)
        .await
        .unwrap()
        .expect("/home");
    let user = world
        .modules
        .libraries
        .generator
        .add_full_node(
            &home,
            &NodeDraft::folder("bob", "bob", PermissionSet::default_for(true)),
        )
        .await
        .unwrap();

    world
        .modules
        .libraries
        .generator
        .add_full_node(
            &user,
            &NodeDraft::file(
                "notes.txt",
                "bob",
                PermissionSet::default_for(false),
                FileData::text("hi"),
            ),
        )
        .await
        .unwrap();

    let mut link = NodeDraft::folder("link", ROOT_USER, PermissionSet::default_for(true));

    link.symlink = "/home/bob".to_owned();
    world
        .modules
        .libraries
        .generator
        .add_full_node(&root, &link)
        .await
        .unwrap();

    (world, computer)
}

pub async fn computers_for_copy() -> (World, ComputerId, ComputerId) {
    let world = generated_world().await;
    let source = world.handle.player();
    let destination = npc(&world).await.id;

    (world, source, destination)
}

pub async fn add_source_file(world: &mut World, computer: ComputerId, data: FileData) {
    let root = world
        .modules
        .core
        .files
        .lookup
        .root(computer)
        .await
        .unwrap()
        .unwrap();

    world
        .modules
        .libraries
        .generator
        .add_full_node(
            &root,
            &NodeDraft::file(
                "source.txt",
                ROOT_USER,
                PermissionSet::default_for(false),
                data,
            ),
        )
        .await
        .unwrap();
}
