use greyhack_behaviour::{MissionKind, NetworkKind, kind_for_ip, random_group_for_mission};
use greyhack_world::{ComputerId, PlayerOptions, World};

/// A world that has generated the networks of the procedural mission `index` of `kind`.
pub async fn mission_world(kind: MissionKind, index: u32) -> (World, Vec<String>) {
    let mut world = World::from_seed("procedural missions", &PlayerOptions::default())
        .await
        .unwrap();
    let group = random_group_for_mission(world.world_seed(), kind, index, 4_242).unwrap();

    for address in &group {
        world.begin().await.unwrap();
        world.router_or_generate(address).await.unwrap();
        world.commit().await.unwrap();
    }

    (world, group)
}

/// Every file path of `computer` with its content.
pub async fn files(world: &World, computer: ComputerId) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut folders = vec![world.root(computer).await.unwrap().unwrap()];

    while let Some(folder) = folders.pop() {
        for child in world.children(&folder).await.unwrap() {
            if child.is_folder() {
                folders.push(child);
            } else if let Some(data) = child.data() {
                found.push((child.path.clone(), data.content.clone()));
            }
        }
    }

    found
}

pub async fn network_files(world: &World, address: &str) -> Vec<(String, String)> {
    let Some(router) = world.router_at(address).await.unwrap() else {
        return Vec::new();
    };
    let mut all = Vec::new();

    for computer in world.devices(router.id).await.unwrap() {
        all.extend(files(world, computer).await);
    }

    all
}

/// An address of a network of `kind` in the world's address space.
pub fn shop_address(world: &World, kind: NetworkKind) -> String {
    (1..=250)
        .map(|third| format!("93.18.{third}.9"))
        .find(|address| kind_for_ip(world.world_seed(), address) == kind)
        .expect("an address of the kind")
}
