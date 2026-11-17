use greyhack_behaviour::{CompanyAlign, FileKind, NetworkKind};
use greyhack_world::{PlayerOptions, World};
use greyhack_world_creation::NetworkOptions;
use greyhack_world_creation::services::zero_day::RumorRequest;
use greyhack_world_libraries::{
    CREDENTIALS_IN_USE, DebugFailure, DebugPurchase, INCORRECT_OFFER_CREDENTIALS,
    OFFER_NOT_AVAILABLE, ONE_OFFER_PER_PLAYER, SALE_CLOSED, USED_OFFER_CREDENTIALS,
};

fn world() -> World {
    greybel_test_support::future::block_on(World::new(&PlayerOptions::default())).unwrap()
}

#[test]
fn should_bind_debug_credentials_to_the_first_buyer_and_expire_offers() {
    let world = world();
    assert!(world.modules.routines.set_date("01/Feb/2024 - 00:00"));
    let debug = &world.modules.libraries.debug;
    let player = debug.player_id();
    let credentials =
        greybel_test_support::future::block_on(debug.get_npc_password(&player)).unwrap();

    assert!(
        greybel_test_support::future::block_on(debug.add_offer_creds(
            &player,
            "",
            1000,
            &credentials.user,
            &credentials.password,
        ))
        .unwrap()
        .is_empty()
    );
    let offer = greybel_test_support::future::block_on(debug.get_best_offer_creds())
        .unwrap()
        .expect("the offer is available");
    assert_eq!(offer.price, 1000);

    let bought = greybel_test_support::future::block_on(debug.buy_offer_creds(
        &offer.seller,
        offer.price,
        &player,
    ))
    .unwrap();
    assert!(matches!(bought, DebugPurchase::Purchased(_)));
    let second = greybel_test_support::future::block_on(debug.add_offer_creds(
        &format!("{player}-other"),
        "",
        1000,
        &credentials.user,
        &credentials.password,
    ))
    .unwrap();
    assert!(second.contains("already been used"));

    assert!(world.modules.routines.set_date("01/Apr/2024 - 00:00"));
    assert!(
        greybel_test_support::future::block_on(debug.get_best_offer_creds())
            .unwrap()
            .is_none()
    );
}

#[test]
fn should_unlock_a_zero_day_zone_without_losing_library_zones() {
    let world = world();
    let computer = world.player();
    let path = "/lib/libssh.so";
    let node = greybel_test_support::future::block_on(
        world.modules.core.files.lookup.file(computer, path, false),
    )
    .unwrap()
    .expect("the player has libssh");
    let before = greybel_test_support::future::block_on(
        world.modules.libraries.generator.full_library_of(&node),
    )
    .unwrap()
    .expect("libssh has a full library")
    .zones
    .len();

    assert!(
        greybel_test_support::future::block_on(
            world
                .modules
                .libraries
                .debug
                .unlock_debug_library(computer, path),
        )
        .unwrap()
    );

    let library = greybel_test_support::future::block_on(
        world.modules.libraries.generator.full_library_of(&node),
    )
    .unwrap()
    .unwrap();
    assert_eq!(library.zones.len(), before + 1);
    assert!(library.zones.iter().any(|zone| !zone.weaknesses.is_empty()));
}

#[test]
fn should_create_the_chain_with_a_weak_aptclient_before_time_advances() {
    let world = world();
    let public_ip = greybel_test_support::future::block_on(
        world.modules.creation.zero_day.create_zero_day_chain(),
    )
    .unwrap()
    .expect("the zero-day chain is generated");
    let router = greybel_test_support::future::block_on(
        world.modules.core.networks.routers.router_at(&public_ip),
    )
    .unwrap()
    .expect("the chain has a router");
    let computers = greybel_test_support::future::block_on(
        world.modules.core.computers.computers_in(router.network),
    )
    .unwrap();
    assert!(!computers.is_empty());

    let aptclient = computers.iter().find_map(|computer| {
        greybel_test_support::future::block_on(world.modules.core.files.lookup.file(
            computer.id,
            "/lib/aptclient.so",
            false,
        ))
        .unwrap()
    });
    assert!(aptclient.is_some(), "the chain has a weak aptclient");

    greybel_test_support::future::block_on(world.tick(20 * 86_400)).unwrap();
    assert!(
        greybel_test_support::future::block_on(world.modules.core.files.lookup.file(
            aptclient.unwrap().computer,
            "/lib/aptclient.so",
            false,
        ),)
        .unwrap()
        .is_some()
    );
}

#[test]
fn should_create_seeded_procedural_missions_deterministically() {
    let first = world();
    let second = world();
    let first_ip = greybel_test_support::future::block_on(
        first
            .modules
            .creation
            .zero_day
            .create_procedural_mission(42),
    )
    .unwrap();
    let second_ip = greybel_test_support::future::block_on(
        second
            .modules
            .creation
            .zero_day
            .create_procedural_mission(42),
    )
    .unwrap();
    assert_eq!(first_ip, second_ip);
    assert!(first_ip.is_some());
}

#[test]
fn should_read_a_mail_account_for_mission_fixtures() {
    let world = world();
    greybel_test_support::future::block_on(
        world
            .modules
            .npcs
            .mail
            .add_mail_account("neo@test.com", "matrix1"),
    )
    .unwrap();
    let account = greybel_test_support::future::block_on(
        world.modules.npcs.mail.mail_account("neo@test.com"),
    )
    .unwrap()
    .unwrap();
    assert_eq!(account.address, "neo@test.com");
    assert!(!account.password_hash.is_empty());
}

#[test]
fn should_open_close_and_reopen_the_lazy_neurobox_rumor() {
    let world = world();
    assert!(world.modules.routines.set_date("01/Feb/2024 - 00:00"));
    let zero_day = &world.modules.creation.zero_day;
    let debug = &world.modules.libraries.debug;
    let player = debug.player_id();

    assert!(
        greybel_test_support::future::block_on(zero_day.is_available_rumor(&player, true)).unwrap()
    );
    let ip = greybel_test_support::future::block_on(debug.rumor_ip()).unwrap();
    let router =
        greybel_test_support::future::block_on(world.modules.core.networks.routers.router_at(&ip))
            .unwrap()
            .expect("the final rumor network exists");
    let network =
        greybel_test_support::future::block_on(world.modules.core.networks.network(router.network))
            .unwrap()
            .unwrap();
    assert_eq!(network.kind, NetworkKind::Neurobox);
    let npcs =
        greybel_test_support::future::block_on(world.modules.npcs.npcs.npcs_in(router.network))
            .unwrap();
    let aligned: Vec<_> = npcs
        .iter()
        .filter(|npc| npc.company_align != CompanyAlign::None)
        .collect();
    assert!((1..=3).contains(&aligned.len()));
    assert!(npcs.len() >= 2);
    assert!(aligned.iter().all(|npc| {
        npc.day_zero_contact.is_some()
            && npcs.iter().any(|other| {
                Some(other.person) != Some(npc.person) && other.company_align == CompanyAlign::None
            })
    }));
    assert!(
        !greybel_test_support::future::block_on(zero_day.is_available_rumor(&player, true))
            .unwrap()
    );

    assert!(world.modules.routines.set_date("01/Mar/2024 - 00:00"));
    assert!(
        !greybel_test_support::future::block_on(zero_day.is_available_rumor(&player, false))
            .unwrap()
    );
    assert!(
        greybel_test_support::future::block_on(debug.rumor_ip())
            .unwrap()
            .is_empty()
    );
    assert!(
        greybel_test_support::future::block_on(world.modules.core.networks.routers.router_at(&ip))
            .unwrap()
            .is_none()
    );
    let cleared = greybel_test_support::future::block_on(debug.rumor()).unwrap();
    assert!(cleared.npc_passwords.is_empty());
    assert!(cleared.requested_credentials.is_empty());
    assert!(cleared.offers.is_empty());

    // Closing the window does not clear the separately persisted per-player request timestamp.
    assert!(world.modules.routines.set_date("01/Apr/2024 - 00:00"));
    assert!(
        greybel_test_support::future::block_on(zero_day.is_available_rumor(&player, true)).unwrap()
    );
    assert!(
        !greybel_test_support::future::block_on(debug.rumor_ip())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn should_apply_the_strict_thirty_day_request_rule_per_player() {
    let world = world();
    assert!(world.modules.routines.set_date("01/Aug/2024 - 00:00"));
    let debug = &world.modules.libraries.debug;

    assert!(greybel_test_support::future::block_on(debug.is_available_rumor("one", true)).unwrap());
    assert!(
        !greybel_test_support::future::block_on(debug.is_available_rumor("one", true)).unwrap()
    );
    assert!(greybel_test_support::future::block_on(debug.is_available_rumor("two", true)).unwrap());
    assert!(world.modules.routines.set_date("31/Aug/2024 - 00:00"));
    assert!(
        !greybel_test_support::future::block_on(debug.is_available_rumor("one", false)).unwrap()
    );
    assert!(world.modules.routines.set_date("01/Oct/2024 - 00:00"));
    assert!(greybel_test_support::future::block_on(debug.is_available_rumor("one", true)).unwrap());
    let players = greybel_test_support::future::block_on(debug.rumor())
        .unwrap()
        .players;
    assert_eq!(players["one"].login_attempts, 0);
    assert_eq!(players["one"].solo_attempts, 0);
}

#[test]
fn should_reuse_credentials_and_bind_them_to_the_first_player() {
    let world = world();
    let debug = &world.modules.libraries.debug;
    let player = debug.player_id();
    let first = greybel_test_support::future::block_on(debug.get_npc_password(&player)).unwrap();
    let again = greybel_test_support::future::block_on(debug.get_npc_password(&player)).unwrap();
    assert_eq!(first, again);
    assert!(first.user.chars().any(char::is_alphabetic));
    assert!(first.user.chars().any(char::is_numeric));
    assert!(!first.password.is_empty());

    assert_eq!(
        greybel_test_support::future::block_on(debug.login_debug_for_player(
            &player,
            &first.user,
            &first.password
        ))
        .unwrap(),
        Ok(())
    );
    assert_eq!(
        greybel_test_support::future::block_on(debug.login_debug_for_player(
            "another-player",
            &first.user,
            &first.password,
        ))
        .unwrap(),
        Err(DebugFailure::CredentialsInUse)
    );
    assert_eq!(
        greybel_test_support::future::block_on(debug.login_debug_for_player(
            "another-player",
            &first.user,
            &first.password,
        ))
        .unwrap(),
        Err(DebugFailure::CredentialsInUse)
    );
    let used = greybel_test_support::future::block_on(debug.buy_offer_creds("missing", 1, &player))
        .unwrap();
    assert!(matches!(used, DebugPurchase::Refused(text) if text == OFFER_NOT_AVAILABLE));
    assert_eq!(
        CREDENTIALS_IN_USE,
        "Error: These credentials are being used by another person."
    );
}

#[test]
fn should_validate_order_and_sell_debug_offers() {
    let world = world();
    assert!(world.modules.routines.set_date("01/Mar/2024 - 00:00"));
    let debug = &world.modules.libraries.debug;
    let player = debug.player_id();
    assert_eq!(
        greybel_test_support::future::block_on(
            debug.add_offer_creds(&player, "", 100, "npc", "pw")
        )
        .unwrap(),
        SALE_CLOSED
    );

    assert!(world.modules.routines.set_date("01/Feb/2024 - 00:00"));
    assert_eq!(
        greybel_test_support::future::block_on(
            debug.add_offer_creds(&player, "", 100, "npc", "pw")
        )
        .unwrap(),
        INCORRECT_OFFER_CREDENTIALS
    );
    let credentials =
        greybel_test_support::future::block_on(debug.get_npc_password(&player)).unwrap();
    assert!(
        greybel_test_support::future::block_on(debug.add_offer_creds(
            &player,
            "",
            100,
            &credentials.user,
            &credentials.password,
        ))
        .unwrap()
        .is_empty()
    );
    let other = greybel_test_support::future::block_on(debug.get_npc_password("other")).unwrap();
    assert!(
        greybel_test_support::future::block_on(debug.add_offer_creds(
            "other",
            "",
            50,
            &other.user,
            &other.password,
        ))
        .unwrap()
        .is_empty()
    );
    assert_eq!(
        greybel_test_support::future::block_on(debug.add_offer_creds(
            &player,
            "",
            100,
            &credentials.user,
            &credentials.password,
        ))
        .unwrap(),
        ONE_OFFER_PER_PLAYER
    );
    assert_eq!(
        greybel_test_support::future::block_on(debug.get_best_offer_creds())
            .unwrap()
            .unwrap()
            .price,
        50
    );
    assert!(matches!(
        greybel_test_support::future::block_on(debug.buy_offer_creds("other", 100, &player)).unwrap(),
        DebugPurchase::Refused(text) if text == OFFER_NOT_AVAILABLE
    ));
    assert!(matches!(
        greybel_test_support::future::block_on(debug.buy_offer_creds("other", 50, &player))
            .unwrap(),
        DebugPurchase::Purchased(_)
    ));
    assert_eq!(
        greybel_test_support::future::block_on(debug.add_offer_creds(
            "third",
            "",
            1,
            &other.user,
            &other.password,
        ))
        .unwrap(),
        USED_OFFER_CREDENTIALS
    );
    assert!(
        greybel_test_support::future::block_on(debug.get_best_offer_creds())
            .unwrap()
            .is_some()
    );
}

#[test]
fn should_mail_a_reachable_first_rumor_and_write_a_chat_with_the_real_state() {
    let world = world();
    assert!(world.modules.routines.set_date("01/Feb/2024 - 00:00"));
    let zero_day = &world.modules.creation.zero_day;
    let RumorRequest::Rumor(mission) =
        greybel_test_support::future::block_on(zero_day.request_zero_day_mission()).unwrap()
    else {
        panic!("the first rumor is available");
    };
    assert!(!mission.lan_ip.is_empty());
    let first = greybel_test_support::future::block_on(
        world
            .modules
            .core
            .networks
            .routers
            .router_at(&mission.public_ip),
    )
    .unwrap()
    .unwrap();
    let network =
        greybel_test_support::future::block_on(world.modules.core.networks.network(first.network))
            .unwrap()
            .unwrap();
    assert!(
        [
            NetworkKind::Police,
            NetworkKind::University,
            NetworkKind::MobileShop
        ]
        .contains(&network.kind)
    );
    let source =
        greybel_test_support::future::block_on(world.modules.npcs.npcs.npcs_in(first.network))
            .unwrap()
            .into_iter()
            .find(|npc| npc.rumor)
            .expect("one NPC carries the rumor");
    let final_ip =
        greybel_test_support::future::block_on(world.modules.libraries.debug.rumor_ip()).unwrap();
    let mail = greybel_test_support::future::block_on(
        world.modules.npcs.mail.mail_account("test@test.com"),
    )
    .unwrap()
    .unwrap();
    assert!(mail.inbox.iter().any(|email| {
        email.subject == "It seems that there are some rumors."
            && email.body.contains(&mission.public_ip)
            && email.body.contains(&mission.lan_ip)
    }));

    let home = greybel_test_support::future::block_on(world.modules.core.files.lookup.folder(
        source.computer,
        &format!("/home/{}", source.user),
        false,
    ))
    .unwrap()
    .unwrap();
    let chat = greybel_test_support::future::block_on(world.modules.core.files.lookup.files(&home))
        .unwrap()
        .into_iter()
        .find_map(|file| {
            let data = file.data()?.clone();
            (data.kind == FileKind::ChatLog && data.content.contains("CONV_RUMOR_")).then_some(data)
        })
        .expect("the rumor NPC has a chat");
    let json: serde_json::Value = serde_json::from_str(&chat.content).unwrap();
    assert!(json["chatID"].as_str().unwrap().starts_with("CONV_RUMOR_"));
    assert_eq!(json["users"]["SEND"].as_str(), Some(source.user.as_str()));
    assert!(!json["idxNpcs"].as_object().unwrap().is_empty());
    let octets: Vec<_> = final_ip.split('.').collect();
    for value in json["idxPublicIPs"].as_object().unwrap().values() {
        let masked: Vec<_> = value.as_str().unwrap().split('.').collect();
        assert_eq!(masked.iter().filter(|part| **part == "XXX").count(), 2);
        for (actual, masked) in octets.iter().zip(masked) {
            assert!(masked == "XXX" || masked == *actual);
        }
    }
}

#[test]
fn should_answer_only_an_aligned_zero_day_contact() {
    let world = world();
    assert!(world.modules.routines.set_date("01/Feb/2024 - 00:00"));
    let zero_day = &world.modules.creation.zero_day;
    assert!(
        greybel_test_support::future::block_on(zero_day.is_available_rumor("player", false))
            .unwrap()
    );
    let ip =
        greybel_test_support::future::block_on(world.modules.libraries.debug.rumor_ip()).unwrap();
    let router =
        greybel_test_support::future::block_on(world.modules.core.networks.routers.router_at(&ip))
            .unwrap()
            .unwrap();
    let npc =
        greybel_test_support::future::block_on(world.modules.npcs.npcs.npcs_in(router.network))
            .unwrap()
            .into_iter()
            .find(|npc| npc.company_align != CompanyAlign::None)
            .unwrap();
    let from = npc.day_zero_contact.clone().unwrap();
    let debug = &world.modules.libraries.debug;
    let wrong = if npc.company_align == CompanyAlign::Good {
        CompanyAlign::Bad
    } else {
        CompanyAlign::Good
    };
    let wrong_text = greybel_test_support::future::block_on(debug.answer_zero_day(
        &ip,
        &npc.user,
        &from,
        wrong,
        &debug.player_id(),
    ))
    .unwrap()
    .unwrap();
    assert!(!wrong_text.contains("$USER"));
    let own =
        greybel_test_support::future::block_on(world.modules.npcs.mail.mail_address(npc.person))
            .unwrap()
            .unwrap();
    assert_eq!(
        greybel_test_support::future::block_on(debug.answer_zero_day(
            &ip,
            &npc.user,
            &own,
            npc.company_align,
            &debug.player_id(),
        ))
        .unwrap()
        .unwrap(),
        "Wait... is this message from my account? Who are you and how did you get in??"
    );
    let answer = greybel_test_support::future::block_on(debug.answer_zero_day(
        &ip,
        &npc.user,
        &from,
        npc.company_align,
        &debug.player_id(),
    ))
    .unwrap()
    .unwrap();
    let credentials =
        greybel_test_support::future::block_on(debug.get_npc_password(&debug.player_id())).unwrap();
    assert!(answer.contains(&credentials.user));
    assert!(answer.contains(&credentials.password));
}

#[test]
fn should_trigger_the_lazy_lifecycle_only_for_a_hackshop_visit() {
    let world = world();
    assert!(world.modules.routines.set_date("01/Feb/2024 - 00:00"));
    let zero_day = &world.modules.creation.zero_day;
    let police = greybel_test_support::future::block_on(
        world
            .modules
            .creation
            .network_generation
            .add_network(&NetworkOptions {
                kind: Some(NetworkKind::Police),
                ..NetworkOptions::default()
            }),
    )
    .unwrap();
    let police_ip =
        greybel_test_support::future::block_on(world.modules.core.networks.routers.router(police))
            .unwrap()
            .unwrap()
            .public_ip;
    assert!(
        !greybel_test_support::future::block_on(zero_day.hack_shop_rumor_available(&police_ip))
            .unwrap()
    );
    assert!(
        greybel_test_support::future::block_on(world.modules.libraries.debug.rumor_ip())
            .unwrap()
            .is_empty()
    );

    let shop = greybel_test_support::future::block_on(
        world
            .modules
            .creation
            .network_generation
            .add_network(&NetworkOptions {
                kind: Some(NetworkKind::HackShop),
                ..NetworkOptions::default()
            }),
    )
    .unwrap();
    let shop_ip =
        greybel_test_support::future::block_on(world.modules.core.networks.routers.router(shop))
            .unwrap()
            .unwrap()
            .public_ip;
    assert!(
        greybel_test_support::future::block_on(zero_day.hack_shop_rumor_available(&shop_ip))
            .unwrap()
    );
    assert!(
        !greybel_test_support::future::block_on(world.modules.libraries.debug.rumor_ip())
            .unwrap()
            .is_empty()
    );
}
