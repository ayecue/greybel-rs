//! Procedural missions (`ProcMissionBuilder.BuildMission`, `MissionGen.GetProceduralMission`,
//! `IPGroupManager.GetRandomGroupForMission`, `Router.PostGenMission`).
//!
//! The game draws the group of a mission with a random start, so each test asks the game for a group and builds the
//! same mission in the simulation from the group's addresses and the world secret, which fix it. The network the game
//! generates and the one the simulation generates are not copies (each computer is drawn from its own generator), so
//! the files of the mission are compared by what the mission fixes: their names, permissions and texts. Both sides
//! draw names and words from the game's own lists (`use_game_word_lists`), because every draw after a name depends on
//! the length of the word drawn.

use greybel_test_support::game::Game;
use greyhack_behaviour::{MissionKind, WorldSeed, mission_for_group};
use greyhack_world::{PlayerOptions, World, procedural_mission_json};
use serde_json::Value;

use crate::support::game::{setup_player, test_client};
use crate::support::parity_missions::{
    mission_files, names_and_permissions, simulated_files, use_game_word_lists,
};

/// Both the game and the simulation build the same procedural mission for a group of addresses: archetype, beats,
/// items, runtime tags, NPCs and file names (`ProcMissionBuilder.BuildMission`).
#[tokio::test]
async fn should_build_the_missions_the_game_builds() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;

    use_game_word_lists(&client).await;
    setup_player(&client).await;

    let secret = client.network_state().await.unwrap().world_secret.to_vec();
    let seed = WorldSeed::new(secret);

    for police in [false, true] {
        let kind = if police {
            MissionKind::PoliceProcedural
        } else {
            MissionKind::Procedural
        };

        for index in 0..kind.count() {
            let reply = client.procedural_mission(police, index).await.unwrap();
            let group: Vec<String> = reply.group.iter().map(ToString::to_string).collect();
            let in_game: Value = serde_json::from_str(&reply.mission).unwrap();

            assert_eq!(mission_for_group(&seed, &group), Some((kind, index)));
            assert_eq!(
                procedural_mission_json(&seed, &group, police),
                Some(in_game),
                "mission {index} of {}",
                kind.name()
            );
        }
    }

    client.shutdown();
}

/// The networks of a mission hold the files its items name, with the permissions and texts the mission fixes, in the
/// game and in the simulation (`NetworkLan.CreateMission`, `Router.PostGenMission`).
#[tokio::test]
async fn should_give_the_mission_networks_the_files_the_game_does() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;

    use_game_word_lists(&client).await;
    setup_player(&client).await;

    let secret = client.network_state().await.unwrap().world_secret.to_vec();

    for police in [false, true] {
        let reply = client.procedural_mission(police, 0).await.unwrap();
        let mission: Value = serde_json::from_str(&reply.mission).unwrap();
        let mut world = World::new(&PlayerOptions::default()).await.unwrap();

        world.set_world_secret(secret.clone());

        for (index, network) in reply.networks.iter().enumerate() {
            let wanted = mission_files(&mission, index);
            let in_game = names_and_permissions(
                network
                    .computers
                    .iter()
                    .flat_map(|computer| computer.files.iter())
                    .map(|file| {
                        (
                            file.path.to_string(),
                            file.permissions.to_string(),
                            file.content.to_string(),
                            file.binary,
                        )
                    }),
                &wanted,
            );

            world.begin().await.unwrap();
            world.router_or_generate(&network.public_ip).await.unwrap();
            world.commit().await.unwrap();

            let router = world.router_at(&network.public_ip).await.unwrap().unwrap();
            let mut files = Vec::new();

            for computer in world.devices(router.id).await.unwrap() {
                files.extend(simulated_files(&world, computer).await);
            }

            let simulated = names_and_permissions(files.into_iter(), &wanted);

            assert_eq!(
                in_game,
                simulated,
                "network {index} of the {} mission",
                if police { "police" } else { "hack shop" }
            );
        }
    }

    client.shutdown();
}
