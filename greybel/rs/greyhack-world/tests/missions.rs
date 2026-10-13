use greyhack_behaviour::{MissionKind, NetworkKind, mission_for_group};
use greyhack_world::{MissionFailure, PlayerOptions, World};

use crate::support::missions::{mission_world, network_files, shop_address};

/// The networks of a hackshop procedural mission hold the files the mission's templates ask for, and the closure's
/// objective (`NetworkLan.CreateMission`, `Router.PostGenMission`).
#[tokio::test]
async fn should_give_the_networks_of_a_procedural_mission_its_files() {
    for index in 0..1 {
        let (world, group) = mission_world(MissionKind::Procedural, index).await;
        let mut texts = 0;

        for address in &group {
            texts += network_files(&world, address)
                .await
                .iter()
                .filter(|(path, content)| {
                    path.starts_with("/home/")
                        && !content.is_empty()
                        && (path.ends_with(".txt")
                            || path.ends_with(".log")
                            || path.ends_with(".pdf"))
                })
                .count();
        }

        assert!(texts > 0, "mission {index} left no files");
    }
}

/// A mission's network does not keep the random content of the computers the mission uses, and keeps the other
/// computers' (`Router.GeneraLanPc`).
#[tokio::test]
async fn should_generate_the_same_world_twice() {
    let (first, group) = mission_world(MissionKind::PoliceProcedural, 1).await;
    let (second, _) = mission_world(MissionKind::PoliceProcedural, 1).await;

    for address in &group {
        assert_eq!(
            network_files(&first, address).await,
            network_files(&second, address).await
        );
    }
}

/// Accepting a mission needs a mission shop and the player's mail account, and mails the contract naming the first
/// network of the mission (`MissionsHelperServer.AcceptMissionServerRpc`).
#[tokio::test]
async fn should_mail_the_contract_of_an_accepted_mission() {
    let player = PlayerOptions {
        mail_address: Some("player@mail.test".to_owned()),
        mail_password: Some("secret".to_owned()),
        ..PlayerOptions::default()
    };
    let mut world = World::from_seed("accepting", &player).await.unwrap();
    let shop = shop_address(&world, NetworkKind::HackShop);
    let supermarket = shop_address(&world, NetworkKind::Supermarket);

    for address in [&shop, &supermarket] {
        world.begin().await.unwrap();
        world.router_or_generate(address).await.unwrap();
        world.commit().await.unwrap();
    }

    assert!(
        world
            .accept_procedural_mission("8.8.8.8")
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        world.accept_procedural_mission(&supermarket).await.unwrap(),
        Some(Err(MissionFailure::NotAMissionShop))
    );

    world.begin().await.unwrap();

    let accepted = world
        .accept_procedural_mission(&shop)
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    world.commit().await.unwrap();

    assert_eq!(
        mission_for_group(world.world_seed(), &accepted.group).map(|(kind, _)| kind),
        Some(MissionKind::Procedural)
    );
    assert!(world.router_at(&accepted.group[0]).await.unwrap().is_some());

    let mails = world
        .refresh_mail("player@mail.test")
        .await
        .unwrap()
        .unwrap();
    let contract = mails
        .iter()
        .find(|mail| mail.subject == "Mission Contract")
        .expect("the contract");

    assert!(contract.body.contains(&accepted.group[0]));
}

/// Accepting a mission without a mail account is refused.
#[tokio::test]
async fn should_refuse_a_mission_without_a_mail_account() {
    let player = PlayerOptions {
        mail_address: None,
        mail_password: None,
        ..PlayerOptions::default()
    };
    let mut world = World::from_seed("accepting", &player).await.unwrap();
    let shop = shop_address(&world, NetworkKind::Police);

    world.begin().await.unwrap();
    world.router_or_generate(&shop).await.unwrap();
    world.commit().await.unwrap();

    assert_eq!(
        world.accept_procedural_mission(&shop).await.unwrap(),
        Some(Err(MissionFailure::NoMailAccount))
    );
}
