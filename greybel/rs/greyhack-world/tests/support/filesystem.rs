use greyhack_behaviour::{FileData, PermissionSet, ROOT_USER};
use greyhack_world::{ComputerId, NodeDraft, PlayerOptions, World};

use crate::support::factory_filesystem::{npc, world as generated_world};

/// Builds a small tree in a real world's filesystem, including a symlink to the user's folder.
pub async fn tree() -> (World, ComputerId) {
    let mut world = World::new(&PlayerOptions::default())
        .await
        .expect("the world generates");
    let computer = world.player();
    let root = world.root(computer).await.unwrap().expect("a root folder");
    let home = world
        .folder(computer, "/home", true)
        .await
        .unwrap()
        .expect("/home");
    let user = world
        .add_node(
            &home,
            &NodeDraft::folder("bob", "bob", PermissionSet::default_for(true)),
        )
        .await
        .unwrap();

    world
        .add_node(
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
    world.add_node(&root, &link).await.unwrap();

    (world, computer)
}

pub async fn computers_for_copy() -> (World, ComputerId, ComputerId) {
    let world = generated_world().await;
    let source = world.player();
    let destination = npc(&world).await.id;

    (world, source, destination)
}

pub async fn add_source_file(world: &mut World, computer: ComputerId, data: FileData) {
    let root = world.root(computer).await.unwrap().unwrap();

    world
        .add_node(
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
