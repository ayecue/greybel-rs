use std::collections::BTreeMap;

use greybel_test_support::future::block_on;
use greybel_test_support::game::Game;
use greybel_test_support::interpreter::constant_clock_options;
use greyhack_test_support_client::TestClient;
use greyhack_test_support_client::protocol::test_support::{
    LanNodeKind, NetworkSnapshot, ZeroDayStateReply,
};
use greyhack_world::{PlayerOptions, World};
use greyhack_world_libraries::{
    DebugNetworkBinding, DebugOffer, DebugPurchase, NpcDebugCredentials, ZeroDayPlayerState,
    ZeroDayRumor,
};

use crate::support::game::{join_fixture, setup_player, smtp_network, ssh_network, test_client};
use crate::support::intrinsics::{run_many_with_world, snapshot_world};

#[tokio::test]
async fn should_match_zero_day_rumor_and_mission_operations() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    setup_player(&client).await;
    let mail_network = smtp_network(&client).await;
    join_fixture(&client, &mail_network).await;
    client.setup_mail("neo", "matrix1").await.unwrap();

    let rumor = client.zero_day_rumor().await.unwrap();
    assert!(!rumor.best_offer.is_empty());
    assert!(!rumor.second_purchase.is_empty());
    let chain = client.create_zero_day_chain().await.unwrap();
    assert!(!chain.group.is_empty());
    let procedural = client.create_procedural_mission().await.unwrap();
    assert!(!procedural.group.is_empty());
    let unlock = ssh_network(&client).await;
    let unlock_snapshot = join_fixture(&client, &unlock).await;
    let local_ip = unlock_snapshot
        .nodes
        .iter()
        .find_map(|node| match &node.node {
            LanNodeKind::Computer(computer)
                if computer
                    .files
                    .iter()
                    .any(|file| file.path == "/lib/libssh.so") =>
            {
                Some(node.local_ip.as_str())
            }
            _ => None,
        })
        .expect("the fresh SSH target has libssh");
    assert!(
        client
            .unlock_debug_library(&unlock.public_ip, local_ip, "/lib/libssh.so")
            .await
            .unwrap()
    );

    tokio::task::spawn_blocking(move || {
        let world = block_on(World::new(&PlayerOptions::default())).unwrap();
        assert!(world.modules.routines.set_date("01/Feb/2024 - 00:00"));
        let debug = &world.modules.libraries.debug;
        let player = debug.player_id();
        let credentials = block_on(debug.get_npc_password(&player)).unwrap();
        assert!(
            block_on(debug.add_offer_creds(
                &player,
                "",
                1000,
                &credentials.user,
                &credentials.password,
            ))
            .unwrap()
            .is_empty()
        );
        let offer = block_on(debug.get_best_offer_creds())
            .unwrap()
            .expect("an offer exists");
        assert!(matches!(
            block_on(debug.buy_offer_creds(&offer.seller, offer.price, &player)).unwrap(),
            DebugPurchase::Purchased(_)
        ));
        let second = block_on(debug.add_offer_creds(
            &format!("{player}-other"),
            "",
            1000,
            &credentials.user,
            &credentials.password,
        ))
        .unwrap();
        assert_eq!(second, rumor.second_purchase);
        assert!(world.modules.routines.set_date("01/Apr/2024 - 00:00"));
        assert!(block_on(debug.get_best_offer_creds()).unwrap().is_none());

        let chain_ip = block_on(world.modules.creation.zero_day.create_zero_day_chain())
            .unwrap()
            .expect("Greybel creates a zero-day chain");
        assert!(!chain_ip.is_empty());
        assert!(
            block_on(
                world
                    .modules
                    .creation
                    .zero_day
                    .create_procedural_mission(42),
            )
            .unwrap()
            .is_some()
        );
        assert!(block_on(debug.unlock_debug_library(world.player(), "/lib/libssh.so")).unwrap());
    })
    .await
    .unwrap();

    client.shutdown();
}

#[tokio::test]
async fn should_match_debug_tools_with_valid_invalid_and_used_npc_credentials() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    setup_player(&client).await;
    client.setup_debug().await.unwrap();
    let snapshot = client.network_state().await.unwrap();
    let opened = open_state(&client, "parity-valid", true, false).await;
    let valid = requested(&opened, "parity-valid");

    compare_debug_tools(
        &client,
        &snapshot,
        &opened,
        &format!(
            "lib = include_lib(\"/lib/metaxploit.so\").load(\"/lib/metaxploit.so\")\nprint lib.debug_tools(\"{}\", \"wrong-password\")",
            valid.0
        ),
    )
    .await;
    compare_debug_tools(
        &client,
        &snapshot,
        &opened,
        &format!(
            "lib = include_lib(\"/lib/metaxploit.so\").load(\"/lib/metaxploit.so\")\nprint typeof(lib.debug_tools(\"{}\", \"{}\"))",
            valid.0, valid.1
        ),
    )
    .await;

    let used = client
        .zero_day_state("parity-used", false, false, false, "", "")
        .await
        .unwrap();
    let used_user = requested(&used, "parity-used");
    let used = client
        .zero_day_state(
            "parity-used",
            false,
            false,
            false,
            &used_user.0,
            "someone-else",
        )
        .await
        .unwrap();

    compare_debug_tools(
        &client,
        &snapshot,
        &used,
        &format!(
            "lib = include_lib(\"/lib/metaxploit.so\").load(\"/lib/metaxploit.so\")\nprint lib.debug_tools(\"{}\", \"{}\")",
            used_user.0, used_user.1
        ),
    )
    .await;

    client.shutdown();
}

#[tokio::test]
async fn should_follow_rumor_network_and_chat_rules_in_the_game_and_greybel() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    setup_player(&client).await;
    let opened = open_state(&client, "parity-network", true, true).await;
    assert!(!opened.ip_address.is_empty());
    assert!(!opened.first_public_ip.is_empty());
    assert!(!opened.first_lan_ip.is_empty());
    assert!(!opened.chats.is_empty());

    for chat in &opened.chats {
        validate_chat(&chat.content, &opened.ip_address, &chat.user);
    }
    assert_router_reachable(&client, &opened.first_public_ip).await;
    assert_router_reachable(&client, &opened.ip_address).await;

    tokio::task::spawn_blocking(|| {
        let world = block_on(World::new(&PlayerOptions::default())).unwrap();
        assert!(world.modules.routines.set_date("01/Feb/2024 - 00:00"));
        let request = block_on(world.modules.creation.zero_day.request_zero_day_mission()).unwrap();
        let greyhack_world_creation::services::zero_day::RumorRequest::Rumor(mission) = request
        else {
            panic!("Greybel has an open rumor");
        };
        assert!(!mission.public_ip.is_empty());
        assert!(!mission.lan_ip.is_empty());
        assert!(mission.body.contains(&mission.public_ip));
        assert!(mission.body.contains(&mission.lan_ip));
        assert!(
            block_on(
                world
                    .modules
                    .core
                    .networks
                    .routers
                    .router_at(&mission.public_ip)
            )
            .unwrap()
            .is_some()
        );
        let greybel_final = block_on(world.modules.libraries.debug.rumor_ip()).unwrap();
        assert!(
            block_on(
                world
                    .modules
                    .core
                    .networks
                    .routers
                    .router_at(&greybel_final)
            )
            .unwrap()
            .is_some()
        );

        let first_router = block_on(
            world
                .modules
                .core
                .networks
                .routers
                .router_at(&mission.public_ip),
        )
        .unwrap()
        .unwrap();
        let rumor_npcs = block_on(world.modules.npcs.npcs.npcs_in(first_router.network)).unwrap();
        let source = rumor_npcs.iter().find(|npc| npc.rumor).expect("rumor NPC");
        let home = block_on(world.modules.core.files.lookup.folder(
            source.computer,
            &format!("/home/{}", source.user),
            false,
        ))
        .unwrap()
        .unwrap();
        let chats = block_on(world.modules.core.files.lookup.files(&home)).unwrap();
        let chat = chats
            .iter()
            .find_map(|file| {
                let data = file.data()?;
                (data.kind == greyhack_behaviour::FileKind::ChatLog
                    && data.content.contains("CONV_RUMOR_"))
                .then_some(data)
            })
            .expect("Greybel writes a rumor chat");
        validate_chat(&chat.content, &greybel_final, &source.user);
    })
    .await
    .unwrap();

    client.shutdown();
}

async fn open_state(
    client: &TestClient,
    player: &str,
    fresh: bool,
    first: bool,
) -> ZeroDayStateReply {
    client
        .zero_day_state(player, true, fresh, first, "", "")
        .await
        .unwrap()
}

fn requested(state: &ZeroDayStateReply, player: &str) -> (String, String) {
    state
        .requested_credentials
        .iter()
        .find(|entry| entry.player_id == player)
        .map(|entry| (entry.user.clone(), entry.password.clone()))
        .expect("the game generated the player's NPC credentials")
}

async fn compare_debug_tools(
    client: &TestClient,
    snapshot: &NetworkSnapshot,
    state: &ZeroDayStateReply,
    source: &str,
) {
    let actual = client
        .execute(source, "/project/test.src", "/project", "test")
        .await
        .unwrap();
    let snapshot = snapshot.clone();
    let rumor = mirrored(state);
    let expected_source = source.to_owned();
    let expected = tokio::task::spawn_blocking(move || {
        let world = snapshot_world("root", &snapshot);
        block_on(world.modules.libraries.debug.restore_rumor(&rumor)).unwrap();
        run_many_with_world(
            &[expected_source.as_str()],
            "root",
            world,
            constant_clock_options,
        )
        .pop()
        .unwrap()
    })
    .await
    .unwrap();

    assert!(!actual.failed, "{}", actual.output);
    assert_eq!(expected.1, None, "{source}");
    assert_eq!(actual.output.trim_end(), expected.0.trim_end(), "{source}");
}

fn mirrored(state: &ZeroDayStateReply) -> ZeroDayRumor {
    ZeroDayRumor {
        npc_passwords: state
            .npc_passwords
            .iter()
            .map(|entry| {
                (
                    entry.user.clone(),
                    NpcDebugCredentials {
                        password: entry.password.clone(),
                        player_id: entry.player_id.clone(),
                    },
                )
            })
            .collect::<BTreeMap<_, _>>(),
        requested_credentials: state
            .requested_credentials
            .iter()
            .map(|entry| {
                (
                    entry.player_id.clone(),
                    greyhack_world_libraries::DebugCredentials {
                        user: entry.user.clone(),
                        password: entry.password.clone(),
                    },
                )
            })
            .collect(),
        offers: state
            .offers
            .iter()
            .map(|entry| {
                (
                    entry.seller.clone(),
                    DebugOffer {
                        seller: entry.seller.clone(),
                        bank: entry.bank.clone(),
                        price: entry.price,
                        user: entry.user.clone(),
                        password: entry.password.clone(),
                        purchased: entry.purchased,
                    },
                )
            })
            .collect(),
        ip_address: state.ip_address.clone(),
        players: state
            .players
            .iter()
            .map(|entry| {
                (
                    entry.player_id.clone(),
                    ZeroDayPlayerState {
                        requested_at: entry.requested_at,
                        login_attempts: entry.login_attempts,
                        solo_attempts: entry.solo_attempts,
                        solo_mail_received: entry.solo_mail_received,
                        failed_unit_testing: entry.failed_unit_testing,
                        network_bindings: entry
                            .network_bindings
                            .iter()
                            .map(|binding| DebugNetworkBinding {
                                user: binding.user.clone(),
                                password: binding.password.clone(),
                                public_ip: binding.public_ip.clone(),
                            })
                            .collect(),
                    },
                )
            })
            .collect(),
    }
}

fn validate_chat(content: &str, final_ip: &str, user: &str) {
    let json: serde_json::Value = serde_json::from_str(content).unwrap();
    let id = json["chatID"].as_str().unwrap();
    assert!(id.starts_with("CONV_RUMOR_"), "{content}");
    assert_eq!(json["users"]["SEND"].as_str(), Some(user), "{content}");
    assert!(!json["users"]["RECEIVE"].as_str().unwrap().is_empty());
    let octets: Vec<&str> = final_ip.split('.').collect();
    for (index, value) in json["idxPublicIPs"]
        .as_object()
        .unwrap()
        .values()
        .enumerate()
    {
        let masked = value.as_str().unwrap();
        let parts: Vec<&str> = masked.split('.').collect();
        assert_eq!(parts.len(), 4);
        assert_eq!(parts.iter().filter(|part| **part == "XXX").count(), 2);
        for (octet, (actual, mask)) in octets.iter().zip(parts.iter()).enumerate() {
            assert!(*mask == "XXX" || mask == actual, "octet {octet}: {masked}");
        }
        assert!(!json["idxNpcs"].as_object().unwrap().is_empty(), "{index}");
    }
}

async fn assert_router_reachable(client: &TestClient, public_ip: &str) {
    let result = client
        .execute(
            &format!("print get_router(\"{public_ip}\") != null"),
            "/project/test.src",
            "/project",
            "test",
        )
        .await
        .unwrap();
    assert!(!result.failed, "{}", result.output);
    assert_eq!(result.output.trim_end(), "1");
}
