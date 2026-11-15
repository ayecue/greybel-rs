//! Helpers of the random content parity test.

use greyhack_behaviour::{DeviceKind, FileKind, ROOT_USER};
use greyhack_test_support_client::protocol::test_support::{LanNodeKind, NetworkSnapshot};
use greyhack_world::{PlayerOptions, World};
use greyhack_world_creation::NetworkOptions;

use super::random_content::{Computer, Entry};

/// The computers of the game's network `snapshot`.
pub fn game_computers(snapshot: &NetworkSnapshot) -> Vec<Computer> {
    snapshot
        .nodes
        .iter()
        .filter_map(|node| match &node.node {
            LanNodeKind::Computer(computer) => Some(Computer {
                standard: computer.kind == "COMPUTER",
                users: computer
                    .users
                    .iter()
                    .map(|user| user.name.clone())
                    .collect(),
                entries: computer
                    .files
                    .iter()
                    .map(|file| Entry {
                        path: file.path.clone(),
                        folder: file.folder,
                        owner: file.owner.clone(),
                        group: file.group.clone(),
                        permissions: file.permissions.clone(),
                        kind: FileKind::of_layout(file.type_file, file.binary),
                        content: file.content.clone(),
                        size: file.size,
                    })
                    .collect(),
            }),
            _ => None,
        })
        .collect()
}

/// The computers of `networks` generated networks of the simulation.
pub async fn simulated_computers(networks: u32) -> Vec<Computer> {
    let world = World::new(&PlayerOptions::default())
        .await
        .expect("the world generates");
    let mut computers = Vec::new();

    for _ in 0..networks {
        let router = world
            .modules
            .creation
            .network_generation
            .add_network(&NetworkOptions::default())
            .await
            .expect("a network is generated");

        for id in world
            .modules
            .core
            .networks
            .topology
            .devices(router)
            .await
            .expect("the devices are read")
        {
            let computer = world
                .modules
                .core
                .networks
                .topology
                .computer(id)
                .await
                .expect("the computer is read")
                .expect("the computer exists");
            let users = world
                .modules
                .core
                .computers
                .users
                .users(id)
                .await
                .expect("the users are read");
            let mut entries = Vec::new();

            for user in users.iter().filter(|user| user.name != ROOT_USER) {
                let Some(home) = world
                    .modules
                    .core
                    .files
                    .lookup
                    .folder(id, &format!("/home/{}", user.name), false)
                    .await
                    .expect("the home is read")
                else {
                    continue;
                };

                for file in world
                    .modules
                    .core
                    .files
                    .lookup
                    .files(&home)
                    .await
                    .expect("the home is listed")
                {
                    let Some(data) = file.data() else {
                        continue;
                    };

                    entries.push(Entry {
                        path: file.path.clone(),
                        folder: false,
                        owner: file.owner.clone(),
                        group: file.group.clone(),
                        permissions: file.permissions.as_str().to_owned(),
                        kind: data.kind,
                        content: data.content.clone(),
                        size: data.size,
                    });
                }
            }

            computers.push(Computer {
                standard: computer.kind == DeviceKind::Computer,
                users: users.into_iter().map(|user| user.name).collect(),
                entries,
            });
        }
    }

    computers
}
