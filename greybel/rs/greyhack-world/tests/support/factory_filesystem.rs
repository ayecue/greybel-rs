use greybel_test_support::world::{computers, world_options};
use greyhack_behaviour::{FileData, ServiceId};
use greyhack_world::{Computer, ComputerId, World};

pub async fn world() -> World {
    World::generate(&world_options(42, 2))
        .await
        .expect("the world generates")
}

pub async fn names(world: &World, computer: ComputerId, path: &str) -> Vec<String> {
    let folder = world
        .modules
        .core
        .files
        .lookup
        .folder(computer, path, false)
        .await
        .expect("the world reads")
        .expect("the folder exists");

    world
        .modules
        .core
        .files
        .lookup
        .children(&folder)
        .await
        .expect("the world reads")
        .into_iter()
        .map(|node| node.name().to_owned())
        .collect()
}

pub async fn data(world: &World, computer: ComputerId, path: &str) -> FileData {
    let node = world
        .modules
        .core
        .files
        .lookup
        .file(computer, path, false)
        .await
        .expect("the world reads")
        .unwrap_or_else(|| panic!("{path} exists"));

    node.data().expect("a file").clone()
}

pub async fn npc(world: &World) -> Computer {
    computers(world)
        .await
        .into_iter()
        .find(|computer| !world.is_player(computer.id))
        .expect("an NPC computer")
}

/// An NPC computer that runs no service whose records root's home has a viewer for.
pub async fn npc_without_record_viewer(world: &World) -> Computer {
    for computer in computers(world).await {
        if world.is_player(computer.id) {
            continue;
        }

        let mut viewer = false;

        for service in [
            ServiceId::Criminals,
            ServiceId::Employees,
            ServiceId::Students,
        ] {
            viewer |= world
                .modules
                .core
                .computers
                .processes
                .has_service(computer.id, service)
                .await
                .expect("the world reads");
        }

        if !viewer {
            return computer;
        }
    }

    panic!("an NPC computer without a record viewer")
}

pub async fn player(world: &World) -> Computer {
    computer(world, world.handle.player()).await
}

async fn computer(world: &World, id: ComputerId) -> Computer {
    world
        .modules
        .core
        .networks
        .topology
        .computer(id)
        .await
        .expect("the world reads")
        .expect("the player")
}
