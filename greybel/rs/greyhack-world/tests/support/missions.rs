use greyhack_behaviour::{MissionKind, NetworkKind, kind_for_ip, random_group_for_mission};
use greyhack_world::{ComputerId, PlayerOptions, World};

/// A world that has generated the networks of the mission `index` of `kind`.
pub async fn mission_world(kind: MissionKind, index: u32) -> (World, Vec<String>) {
    let world = World::from_seed("procedural missions", &PlayerOptions::default())
        .await
        .unwrap();
    let group = random_group_for_mission(&world.handle.world_seed(), kind, index, 4_242).unwrap();

    if kind == MissionKind::ZeroDay {
        world.modules.routines.set_date("01/Feb/2000 - 06:00");
    }

    for address in &group {
        world.begin().await.unwrap();
        world
            .modules
            .creation
            .network_generation
            .router_or_generate(address)
            .await
            .unwrap();
        world.commit().await.unwrap();
    }

    (world, group)
}

/// Every file path of `computer` with its content.
pub async fn files(world: &World, computer: ComputerId) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut folders = vec![
        world
            .modules
            .core
            .files
            .lookup
            .root(computer)
            .await
            .unwrap()
            .unwrap(),
    ];

    while let Some(folder) = folders.pop() {
        for child in world
            .modules
            .core
            .files
            .lookup
            .children(&folder)
            .await
            .unwrap()
        {
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
    let Some(router) = world
        .modules
        .core
        .networks
        .routers
        .router_at(address)
        .await
        .unwrap()
    else {
        return Vec::new();
    };
    let mut all = Vec::new();

    for computer in world
        .modules
        .core
        .networks
        .topology
        .devices(router.id)
        .await
        .unwrap()
    {
        all.extend(files(world, computer).await);
    }

    all
}

/// An address of a network of `kind` in the world's address space.
pub fn shop_address(world: &World, kind: NetworkKind) -> String {
    (1..=250)
        .map(|third| format!("93.18.{third}.9"))
        .find(|address| kind_for_ip(&world.handle.world_seed(), address) == kind)
        .expect("an address of the kind")
}

/// The file names of the mission's text file items for network `network` that the mission puts in the home of the
/// account that owns them, and every file name its items name.
pub fn mission_file_names(
    mission: &serde_json::Value,
    network: usize,
) -> (Vec<String>, Vec<String>) {
    let mut homes = Vec::new();
    let mut all = Vec::new();

    for computer in mission["parts"][network].as_array().into_iter().flatten() {
        for item in computer["items"].as_array().into_iter().flatten() {
            if item["type"] != 3 {
                continue;
            }

            let name = item["fileName"].as_str().unwrap_or_default().to_owned();

            if item["randomPath"] == true && item["isWeb"] != true && !name.is_empty() {
                homes.push(name.clone());
            }

            all.push(name);
        }
    }

    (homes, all)
}

/// The kinds of mission the game keeps in lists.
pub const LISTED: [MissionKind; 5] = [
    MissionKind::Secret,
    MissionKind::Grey,
    MissionKind::Black,
    MissionKind::White,
    MissionKind::ZeroDay,
];
