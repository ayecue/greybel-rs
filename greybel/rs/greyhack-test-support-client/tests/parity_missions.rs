//! Missions of every kind (`MissionGen.GetMission`, `MissionGen.GetProceduralMission`, `ProcMissionBuilder.BuildMission`,
//! `IPGroupManager.GetRandomGroupForMission`, `NetworkLan.CreateMission`, `Router.PostGenMission`).
//!
//! The game draws the group of a mission with a random start, so each test asks the game for a group and builds the
//! same mission in the simulation from the group's addresses and the world secret, which fix it. The network the game
//! generates and the one the simulation generates are not copies (each computer is drawn from its own generator), so
//! the files of the mission are compared by what the mission fixes: their names, permissions and texts. Both sides
//! draw names and words from the game's own lists (`use_game_word_lists`), because every draw after a name depends on
//! the length of the word drawn.

use greybel_test_support::game::Game;
use greyhack_behaviour::{MissionKind, WorldSeed, mission_for_group};
use greyhack_world::{PlayerOptions, World};
use greyhack_world_missions::mission_json;
use serde_json::Value;

use crate::support::game::{setup_player, test_client};
use crate::support::parity_missions::{
    GENERATED, KINDS, Observed, mission_files, mission_found, mission_rule_count, mission_websites,
    simulated_files, use_game_word_lists, zero_day_services,
};

/// Both the game and the simulation build the same mission of every kind for a group of addresses: the authored
/// mission with its NPCs drawn from the seed of the group, or the procedural archetype, beats, items, runtime tags,
/// NPCs and file names (`MissionGen.GetMission`, `ProcMissionBuilder.BuildMission`).
#[tokio::test]
async fn should_build_the_missions_the_game_builds() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;

    use_game_word_lists(&client).await;
    setup_player(&client).await;

    let secret = client.network_state().await.unwrap().world_secret.to_vec();
    let seed = WorldSeed::new(secret);

    for kind in KINDS {
        for index in 0..kind.count() {
            let reply = client.mission(kind.name(), index).await.unwrap();
            let group: Vec<String> = reply.group.iter().map(ToString::to_string).collect();
            let in_game: Value = serde_json::from_str(&reply.mission).unwrap();

            assert_eq!(mission_for_group(&seed, &group), Some((kind, index)));
            assert_eq!(
                mission_json(&seed, &group, kind, index),
                Some(in_game),
                "mission {index} of {}",
                kind.name()
            );
        }
    }

    client.shutdown();
}

/// The networks of a mission hold the files its items name, with the permissions and texts the mission fixes, in the
/// game and in the simulation (`NetworkLan.CreateMission`, `Router.PostGenMission`), for every kind of mission.
#[tokio::test]
async fn should_give_the_mission_networks_the_files_the_game_does() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;

    use_game_word_lists(&client).await;
    setup_player(&client).await;

    let secret = client.network_state().await.unwrap().world_secret.to_vec();

    for (kind, mission_index) in GENERATED {
        let reply = client.mission(kind.name(), mission_index).await.unwrap();
        let mission: Value = serde_json::from_str(&reply.mission).unwrap();
        let world = World::new(&PlayerOptions::default()).await.unwrap();

        world.set_world_secret(secret.clone());
        assert!(world.modules.routines.set_date(&reply.current_date));

        for (index, network) in reply.networks.iter().enumerate() {
            let wanted = mission_files(&mission, index);
            let websites = mission_websites(&mission, index);
            let in_game = mission_found(
                network
                    .computers
                    .iter()
                    .flat_map(|computer| computer.files.iter())
                    .filter(|file| !file.folder)
                    .map(|file| Observed {
                        path: file.path.clone(),
                        owner: file.owner.clone(),
                        permissions: file.permissions.clone(),
                        content: file.content.clone(),
                        binary: file.binary,
                    }),
                &wanted,
                &websites,
            );

            world.begin().await.unwrap();
            world
                .modules
                .creation
                .network_generation
                .router_or_generate(&network.public_ip)
                .await
                .unwrap();
            world.commit().await.unwrap();

            let router = world
                .modules
                .core
                .networks
                .routers
                .router_at(&network.public_ip)
                .await
                .unwrap()
                .unwrap();
            let mut files = Vec::new();
            let computers = world
                .modules
                .core
                .networks
                .topology
                .devices(router.id)
                .await
                .unwrap();

            for computer in &computers {
                files.extend(simulated_files(&world, *computer).await);
            }

            let simulated = mission_found(files.into_iter(), &wanted, &websites);

            // The two sides generate networks of different sizes (each draws its own computers), so the count of
            // computers is only reported.
            assert_eq!(
                in_game,
                simulated,
                "network {index} of mission {mission_index} of {} ({} computers in the game, {} in the simulation)",
                kind.name(),
                network.computers.len(),
                computers.len() + 1
            );
        }
    }

    client.shutdown();
}

/// The firewall rules a mission gives the main router of a network come first and are the game's, repeated as the
/// game repeats them, and the zero-day services are forwarded from their own ports
/// (`NetworkLan.ConfigureChainMission`, `NetworkLan.ConfigurePendingFwItems`, `NetworkLan.ConfigMainFirewall`).
#[tokio::test]
async fn should_give_the_main_routers_the_rules_and_forwards_the_game_does() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;

    use_game_word_lists(&client).await;
    setup_player(&client).await;

    let secret = client.network_state().await.unwrap().world_secret.to_vec();

    for (kind, mission_index) in [(MissionKind::Secret, 4), (MissionKind::ZeroDay, 0)] {
        let reply = client.mission(kind.name(), mission_index).await.unwrap();
        let mission: Value = serde_json::from_str(&reply.mission).unwrap();
        let world = World::new(&PlayerOptions::default()).await.unwrap();

        world.set_world_secret(secret.clone());
        assert!(world.modules.routines.set_date(&reply.current_date));

        for (index, network) in reply.networks.iter().enumerate() {
            world.begin().await.unwrap();
            world
                .modules
                .creation
                .network_generation
                .router_or_generate(&network.public_ip)
                .await
                .unwrap();
            world.commit().await.unwrap();

            let router = world
                .modules
                .core
                .networks
                .routers
                .router_at(&network.public_ip)
                .await
                .unwrap()
                .unwrap();
            let simulated: Vec<(String, String, String, String)> = world
                .modules
                .core
                .networks
                .router_config
                .firewall_rules(router.id)
                .await
                .unwrap()
                .into_iter()
                .map(|rule| {
                    let any = |value: Option<String>| value.unwrap_or_else(|| "Any".to_owned());

                    (
                        if rule.allow { "ALLOW" } else { "DENY" }.to_owned(),
                        any(rule.port.map(|port| port.to_string())),
                        any(rule.source),
                        any(rule.destination),
                    )
                })
                .collect();
            let in_game: Vec<(String, String, String, String)> = network
                .firewall_rules
                .iter()
                .map(|rule| {
                    (
                        rule.action.clone(),
                        rule.port.clone(),
                        rule.source.clone(),
                        rule.destination.clone(),
                    )
                })
                .collect();
            let mission_rules = mission_rule_count(&mission, index);

            assert!(in_game.len() >= mission_rules);
            assert_eq!(
                simulated[..mission_rules],
                in_game[..mission_rules],
                "network {index} of mission {mission_index} of {}",
                kind.name()
            );

            // `ConfigMainFirewall`: a main router with rules allows each forwarded port of the table.
            assert_eq!(simulated.is_empty(), in_game.is_empty());

            if kind != MissionKind::ZeroDay || index == 0 {
                continue;
            }

            for service in zero_day_services(&mission, index) {
                let forwards: Vec<&_> = network
                    .ports
                    .iter()
                    .filter(|port| port.number == service.default_port())
                    .collect();

                assert_eq!(forwards.len(), 1, "{service:?} of network {index}");
            }
        }
    }

    client.shutdown();
}
