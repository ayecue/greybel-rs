use greyhack_behaviour::{
    DEFAULT_REPOSITORY_PORT, FileData, NetworkKind, PermissionSet, ROOT_USER,
};
use greyhack_world::{PlayerOptions, World};
use greyhack_world_core::{ComputerId, Hardware, MinerHardware, NetCard};
use greyhack_world_libraries::{ComputerDraft, FileSystemDraft, NodeDraft};

use super::generation::add_typed_network;
use super::layout::startup_files;

/// Adds `node` to `folder` of a laid-out computer, with the folders above it.
pub fn put(filesystem: &mut FileSystemDraft, folder: &str, node: NodeDraft) {
    let mut parent = filesystem.root();

    for name in folder.split('/').filter(|name| !name.is_empty()) {
        let existing = filesystem
            .children(parent)
            .into_iter()
            .find(|child| filesystem.node(*child).name == name);

        parent = existing.unwrap_or_else(|| {
            filesystem.add(
                parent,
                NodeDraft::folder(name, ROOT_USER, PermissionSet::default_for(true)),
            )
        });
    }

    filesystem.add(parent, node);
}

/// A root-owned file.
pub fn file(name: &str, data: FileData) -> NodeDraft {
    NodeDraft::file(name, ROOT_USER, PermissionSet::default_for(false), data)
}

/// Gives a laid-out computer what `Networking.CheckServiceOnline` needs: its startup files and an active card.
pub fn online(computer: &mut ComputerDraft) {
    let cards = vec![NetCard {
        active: true,
        ..NetCard::new(false, false)
    }];

    computer.hardware = Hardware::standard(2048, 10_000.0, &MinerHardware::default(), cards);
    startup_files(computer);
}

/// A world with a generated HackShop, and the player's sources listing its repository.
pub async fn world_with_shop_source() -> (World, ComputerId, ComputerId) {
    let mut world = World::new(&PlayerOptions::default()).await.unwrap();
    let (router, shop) = add_typed_network(&mut world, NetworkKind::HackShop, "shop.test").await;
    let address = world
        .modules
        .core
        .networks
        .routers
        .router(router)
        .await
        .unwrap()
        .unwrap()
        .public_ip;
    let player = world.handle.player();

    world
        .modules
        .services
        .apt
        .add_apt_repository(player, &address, DEFAULT_REPOSITORY_PORT)
        .await
        .unwrap()
        .unwrap();
    world
        .modules
        .services
        .apt
        .update_apt_packages(player, &[])
        .await
        .unwrap();

    (world, player, shop)
}

pub async fn shop_file_names(world: &World, shop: ComputerId) -> Vec<String> {
    let downloads = world
        .modules
        .core
        .files
        .lookup
        .folder(shop, "/Public/htdocs/downloads", false)
        .await
        .unwrap()
        .unwrap();

    world
        .modules
        .core
        .files
        .lookup
        .files(&downloads)
        .await
        .unwrap()
        .into_iter()
        .map(|node| node.name().to_owned())
        .collect()
}
