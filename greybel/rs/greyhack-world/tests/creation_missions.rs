use greyhack_behaviour::{MissionKind, is_lan_ip, random_group_for_mission};
use greyhack_world::{PlayerOptions, World};
use greyhack_world_missions::request_for;

/// The contact the starter mission mail of the network's NPCs names is the first network of the group, or the first
/// mission computer in that network, for every kind of mission (`Persona.GenerateMailStarterMission`).
#[tokio::test]
async fn should_name_the_contact_of_the_starter_mail_in_every_kind_of_mission() {
    for kind in [
        MissionKind::Secret,
        MissionKind::Grey,
        MissionKind::Black,
        MissionKind::White,
        MissionKind::ZeroDay,
        MissionKind::Procedural,
        MissionKind::PoliceProcedural,
    ] {
        let world = World::from_seed("starter mails", &PlayerOptions::default())
            .await
            .unwrap();
        let group =
            random_group_for_mission(world.handle.world().world_seed(), kind, 0, 99).unwrap();

        world.modules.routines.set_date("01/Feb/2000 - 06:00");

        for address in &group {
            let Some(request) = request_for(world.handle.world().world_seed(), address, true)
            else {
                continue;
            };

            world.begin().await.unwrap();
            world
                .modules
                .creation
                .network_generation
                .router_or_generate(address)
                .await
                .unwrap();
            world.commit().await.unwrap();

            let router = world
                .modules
                .core
                .networks
                .routers
                .router_at(address)
                .await
                .unwrap()
                .unwrap();
            let contact = world
                .modules
                .core
                .networks
                .mission_contact(router.network)
                .await
                .unwrap();
            let contact = contact.expect("a mission network names a contact");

            if request.index == 0 {
                assert!(is_lan_ip(&contact), "{kind:?}: {contact}");
            } else {
                assert_eq!(contact, group[0], "{kind:?}");
            }
        }
    }
}

/// A network a mission was not placed in names no contact.
#[tokio::test]
async fn should_name_no_contact_in_a_network_without_a_mission() {
    let world = World::from_seed("starter mails", &PlayerOptions::default())
        .await
        .unwrap();
    let group = random_group_for_mission(
        world.handle.world().world_seed(),
        MissionKind::NoMission,
        0,
        99,
    )
    .unwrap();

    world.begin().await.unwrap();
    world
        .modules
        .creation
        .network_generation
        .router_or_generate(&group[0])
        .await
        .unwrap();
    world.commit().await.unwrap();

    let router = world
        .modules
        .core
        .networks
        .routers
        .router_at(&group[0])
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        world
            .modules
            .core
            .networks
            .mission_contact(router.network)
            .await
            .unwrap(),
        None
    );
}

/// A group of the zero-day kind has no mission while its rumor is closed, and has one while it is open
/// (`NetworkLan.ConfigureMissions`, `ZeroDaySystem.IsRumorTime`).
#[tokio::test]
async fn should_only_place_a_zero_day_mission_while_its_rumor_is_open() {
    let day = 24 * 60 * 60;

    for (seconds, open) in [
        (0, false),
        (30 * day, true),
        (60 * day, false),
        (90 * day, true),
    ] {
        let world = World::from_seed("zero day", &PlayerOptions::default())
            .await
            .unwrap();
        let group = random_group_for_mission(
            world.handle.world().world_seed(),
            MissionKind::ZeroDay,
            0,
            99,
        )
        .unwrap();

        world.modules.routines.advance_clock(seconds);

        for address in &group {
            if request_for(world.handle.world().world_seed(), address, true).is_none() {
                continue;
            }

            world.begin().await.unwrap();
            world
                .modules
                .creation
                .network_generation
                .router_or_generate(address)
                .await
                .unwrap();
            world.commit().await.unwrap();

            let router = world
                .modules
                .core
                .networks
                .routers
                .router_at(address)
                .await
                .unwrap()
                .unwrap();
            let contact = world
                .modules
                .core
                .networks
                .mission_contact(router.network)
                .await
                .unwrap();

            assert_eq!(
                contact.is_some(),
                open,
                "{address} after {} days",
                seconds / day
            );
        }
    }
}
