use greyhack_behaviour::{MissionKind, NetworkKind, ServiceId, mission_for_group};
use greyhack_world::{PlayerOptions, World};
use greyhack_world_missions::mission_json;
use greyhack_world_player::MissionFailure;

use crate::support::missions::{
    LISTED, mission_file_names, mission_world, network_files, shop_address,
};

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
    let world = World::from_seed("accepting", &player).await.unwrap();
    let shop = shop_address(&world, NetworkKind::HackShop);
    let supermarket = shop_address(&world, NetworkKind::Supermarket);

    for address in [&shop, &supermarket] {
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

    assert!(
        world
            .modules
            .player
            .mission_offers
            .accept_procedural_mission("8.8.8.8")
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        world
            .modules
            .player
            .mission_offers
            .accept_procedural_mission(&supermarket)
            .await
            .unwrap(),
        Some(Err(MissionFailure::NotAMissionShop))
    );

    world.begin().await.unwrap();

    let accepted = world
        .modules
        .player
        .mission_offers
        .accept_procedural_mission(&shop)
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    world.commit().await.unwrap();

    assert_eq!(
        mission_for_group(&world.handle.world_seed(), &accepted.group).map(|(kind, _)| kind),
        Some(MissionKind::Procedural)
    );
    assert!(
        world
            .modules
            .core
            .networks
            .routers
            .router_at(&accepted.group[0])
            .await
            .unwrap()
            .is_some()
    );

    let mails = world
        .modules
        .npcs
        .mail
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
    let world = World::from_seed("accepting", &player).await.unwrap();
    let shop = shop_address(&world, NetworkKind::Police);

    world.begin().await.unwrap();
    world
        .modules
        .creation
        .network_generation
        .router_or_generate(&shop)
        .await
        .unwrap();
    world.commit().await.unwrap();

    assert_eq!(
        world
            .modules
            .player
            .mission_offers
            .accept_procedural_mission(&shop)
            .await
            .unwrap(),
        Some(Err(MissionFailure::NoMailAccount))
    );
}

/// Every kind of mission puts the files its items name in the home of an account of the computers it takes, in each
/// network of its group (`NetworkLan.CreateMission`, `Router.PostGenMission`).
#[tokio::test]
async fn should_give_the_networks_of_every_kind_of_mission_its_files() {
    for kind in LISTED {
        for index in 0..kind.count().min(2) {
            let (world, group) = mission_world(kind, index).await;
            let mission = mission_json(&world.handle.world_seed(), &group, kind, index).unwrap();

            let parts = mission["parts"].as_array().unwrap().len();

            for (network, address) in group.iter().enumerate().take(parts) {
                let (homes, _) = mission_file_names(&mission, network);
                let present: Vec<String> = network_files(&world, address)
                    .await
                    .into_iter()
                    .map(|(path, _)| path.rsplit('/').next().unwrap_or_default().to_owned())
                    .collect();

                for name in homes {
                    assert!(
                        present.contains(&name),
                        "{name} of network {network} of mission {index} of {}",
                        kind.name()
                    );
                }
            }
        }
    }
}

/// The later networks of a zero-day mission forward the services of the mission from the service's own port
/// (`NetworkLan.ConfigureChainMission`).
#[tokio::test]
async fn should_forward_the_services_of_a_zero_day_mission_from_their_own_ports() {
    let (world, group) = mission_world(MissionKind::ZeroDay, 0).await;
    let mission =
        mission_json(&world.handle.world_seed(), &group, MissionKind::ZeroDay, 0).unwrap();
    let mut checked = 0;

    for (network, part) in mission["parts"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .skip(1)
    {
        let router = world
            .modules
            .core
            .networks
            .routers
            .router_at(&group[network])
            .await
            .unwrap()
            .unwrap();
        let forwards = world
            .modules
            .core
            .networks
            .router_config
            .port_forwards(router.id)
            .await
            .unwrap();

        for item in part
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|computer| computer["items"].as_array().unwrap())
            .filter(|item| item["type"] == 4)
        {
            let service = ServiceId::from_code(item["service"].as_i64().unwrap()).unwrap();

            assert!(
                forwards
                    .iter()
                    .any(|forward| forward.number == service.default_port()
                        && forward.port.service == service),
                "{service:?} of network {network}"
            );

            checked += 1;
        }
    }

    assert!(checked > 0);
}

/// A hidden mission that adds firewall rules gives them to the main router of its network, and the router allows
/// each forwarded port (`NetworkLan.ConfigureChainMission`, `NetworkLan.ConfigMainFirewall`).
#[tokio::test]
async fn should_give_the_main_router_the_firewall_rules_of_the_mission() {
    let (world, group) = mission_world(MissionKind::Secret, 4).await;
    let router = world
        .modules
        .core
        .networks
        .routers
        .router_at(&group[1])
        .await
        .unwrap()
        .unwrap();
    let rules = world
        .modules
        .core
        .networks
        .router_config
        .firewall_rules(router.id)
        .await
        .unwrap();
    let forwards = world
        .modules
        .core
        .networks
        .router_config
        .port_forwards(router.id)
        .await
        .unwrap();

    assert_eq!(rules[0].port, Some(80));
    assert!(rules[0].allow);
    assert!(
        rules
            .iter()
            .any(|rule| rule.port == Some(22) && !rule.allow)
    );
    assert!(
        rules
            .iter()
            .any(|rule| rule.source.as_deref() == Some(group[2].as_str()))
    );

    for forward in forwards
        .iter()
        .filter(|forward| forward.lan_ip != router.local_ip)
    {
        assert!(rules.iter().any(|rule| rule.allow
            && rule.port == Some(forward.number)
            && rule.destination.as_deref() == Some(forward.lan_ip.as_str())));
    }
}

/// A network the mission gives no firewall rules keeps the main router without any.
#[tokio::test]
async fn should_leave_the_main_router_of_other_missions_without_rules() {
    let (world, group) = mission_world(MissionKind::Grey, 0).await;

    for address in &group {
        let router = world
            .modules
            .core
            .networks
            .routers
            .router_at(address)
            .await
            .unwrap()
            .unwrap();

        assert!(
            world
                .modules
                .core
                .networks
                .router_config
                .firewall_rules(router.id)
                .await
                .unwrap()
                .is_empty()
        );
    }
}

/// A computer the mission gives the http service keeps the mission's web file over the website of the service: its
/// text id is empty, so the website the service writes is empty, and `Router.PostGenMission` replaces it with the web
/// item's text (`NetworkLan.ConfigureChainMission`, `Computer.ConfiguringServices`).
#[tokio::test]
async fn should_replace_the_empty_website_of_a_mission_service_by_the_web_item() {
    let (world, group) = mission_world(MissionKind::Secret, 2).await;
    let files = network_files(&world, &group[1]).await;
    let party = greyhack_world_assets::content_file("PartyMission");

    assert!(
        files
            .iter()
            .any(|(path, content)| path == "/Public/htdocs/website.html" && content == party)
    );
    assert!(
        !files
            .iter()
            .any(|(path, content)| path == "/Public/htdocs/website.html" && content.is_empty())
    );
}

/// A mission puts its files in the folders the services of its computer create, and over the files the services
/// write, because the game configures the services of a computer before it adds the mission's files
/// (`Router.GeneraLanPc`, `Router.PostGenMission`).
#[tokio::test]
async fn should_add_the_files_of_a_mission_after_the_services_of_its_computers() {
    let (world, group) = mission_world(MissionKind::Secret, 9).await;
    let service_files = network_files(&world, &group[1]).await;
    let project = network_files(&world, &group[2]).await;
    let repod = service_files
        .iter()
        .find(|(path, _)| path == "/server/conf/repod.conf")
        .expect("the mission's repod.conf");

    assert!(
        service_files
            .iter()
            .any(|(path, _)| path == "/server/DungeonSeeker")
    );
    assert!(
        project
            .iter()
            .any(|(path, _)| path == "/server/Project.pdf")
    );
    assert!(!repod.1.contains("helloworld"));
}
