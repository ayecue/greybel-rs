use greyhack_behaviour::{JobRole, NetworkKind};
use greyhack_world_npcs::{FireFailure, HireFailure};

use crate::support::npcs_jobs::{public_ip, world_with};

#[tokio::test]
async fn should_give_a_police_networks_npcs_police_roles_an_admin_and_the_camera_credentials() {
    // Arrange
    let (world, network) = world_with(NetworkKind::Police).await;

    // Act
    let npcs = world.modules.npcs.npcs.npcs_in(network).await.unwrap();
    let admin = world
        .modules
        .npcs
        .npcs
        .network_admin_person(network)
        .await
        .unwrap();
    let credentials = world
        .modules
        .smart_devices
        .traffic
        .traffic_credentials(network)
        .await
        .unwrap()
        .unwrap();

    // Assert
    assert!(!npcs.is_empty());
    for npc in &npcs {
        if Some(npc.person) == admin {
            assert_eq!(npc.job_role, JobRole::Admin);
        } else {
            assert!(
                JobRole::available_for(true).contains(&npc.job_role),
                "{npc:?}"
            );
        }

        if let Some(plate) = &npc.license_plate {
            let address = public_ip(&world, network).await;

            assert_eq!(
                Some(plate),
                greyhack_behaviour::license_plate(
                    world.handle.world().world_seed(),
                    &address,
                    &npc.user
                )
                .as_ref()
            );
            assert_eq!(
                greyhack_behaviour::license_plate_address(world.handle.world().world_seed(), plate),
                address
            );
        }
    }

    assert!(
        JobRole::POLICE.contains(&credentials.job_role) || Some(credentials.person) == admin,
        "{credentials:?}"
    );
}

#[tokio::test]
async fn should_log_a_fired_npc_out_for_good() {
    // Arrange
    let (world, network) = world_with(NetworkKind::Supermarket).await;
    let address = public_ip(&world, network).await;
    let npc = world
        .modules
        .npcs
        .npcs
        .npcs_in(network)
        .await
        .unwrap()
        .remove(0);
    let at_work = (world.handle.time_seconds()..world.handle.time_seconds() + 7 * 86_400)
        .step_by(60)
        .find(|at| npc.schedule.is_working(*at))
        .expect("the NPC works this week");

    world
        .tick(at_work - world.handle.time_seconds())
        .await
        .unwrap();

    let online_at_work = world
        .modules
        .npcs
        .npcs
        .npc(npc.person)
        .await
        .unwrap()
        .unwrap()
        .online;

    // Act
    world
        .modules
        .npcs
        .jobs
        .fire_npc(&address, &npc.user)
        .await
        .unwrap();
    world.tick(60).await.unwrap();

    let online_after = world
        .modules
        .npcs
        .npcs
        .npc(npc.person)
        .await
        .unwrap()
        .unwrap()
        .online;

    world.tick(7 * 86_400).await.unwrap();

    let online_a_week_later = world
        .modules
        .npcs
        .npcs
        .npc(npc.person)
        .await
        .unwrap()
        .unwrap()
        .online;

    // Assert
    assert!(online_at_work);
    assert!(!online_after);
    assert!(!online_a_week_later);
}

#[tokio::test]
async fn should_fire_an_npc_who_stays_alive_and_opens_their_position() {
    // Arrange
    let (world, network) = world_with(NetworkKind::Supermarket).await;
    let address = public_ip(&world, network).await;
    let npc = world
        .modules
        .npcs
        .npcs
        .npcs_in(network)
        .await
        .unwrap()
        .remove(0);

    // Act
    let unknown = world
        .modules
        .npcs
        .jobs
        .fire_npc(&address, "nobody")
        .await
        .unwrap();
    let fired = world
        .modules
        .npcs
        .jobs
        .fire_npc(&address, &npc.user)
        .await
        .unwrap();
    let row = world
        .modules
        .npcs
        .npcs
        .npc(npc.person)
        .await
        .unwrap()
        .unwrap();
    let alive = row.dead_at.is_none();
    let again = world
        .modules
        .npcs
        .jobs
        .fire_npc(&address, &npc.user)
        .await
        .unwrap();
    let hired = world
        .modules
        .npcs
        .jobs
        .hire_npc(&address, npc.job_role)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let vacancy = world
        .modules
        .npcs
        .npcs
        .npc(hired)
        .await
        .unwrap()
        .unwrap()
        .computer;

    // Assert
    assert_eq!(unknown, Some(Err(FireFailure::UnknownPerson)));
    assert_eq!(fired, Some(Ok(())));
    assert!(row.fired);
    assert!(alive);
    assert_eq!(again, Some(Err(FireFailure::Unavailable)));
    assert_eq!(vacancy, npc.computer);
}

#[tokio::test]
async fn should_hire_an_npc_onto_an_open_position_who_cant_be_fired_for_a_month() {
    // Arrange
    let (world, network) = world_with(NetworkKind::Supermarket).await;
    let address = public_ip(&world, network).await;
    let npc = world
        .modules
        .npcs
        .npcs
        .npcs_in(network)
        .await
        .unwrap()
        .remove(0);
    let role = npc.job_role;

    // Act
    let police = world
        .modules
        .npcs
        .jobs
        .hire_npc(&address, JobRole::Sergeant)
        .await
        .unwrap();
    let without_vacancy = world
        .modules
        .npcs
        .jobs
        .hire_npc(&address, role)
        .await
        .unwrap();

    world
        .modules
        .npcs
        .jobs
        .fire_npc(&address, &npc.user)
        .await
        .unwrap();

    let hired = world
        .modules
        .npcs
        .jobs
        .hire_npc(&address, role)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let row = world.modules.npcs.npcs.npc(hired).await.unwrap().unwrap();
    let account = world
        .modules
        .core
        .computers
        .user(npc.computer, &row.user)
        .await
        .unwrap();
    let recently = world
        .modules
        .npcs
        .jobs
        .fire_npc(&address, &row.user)
        .await
        .unwrap();

    world.handle.world_mut().time_seconds += 31 * greyhack_behaviour::SECONDS_PER_DAY;

    let later = world
        .modules
        .npcs
        .jobs
        .fire_npc(&address, &row.user)
        .await
        .unwrap();

    // Assert
    assert_eq!(police, Some(Err(HireFailure::InvalidJob)));
    assert_eq!(without_vacancy, Some(Err(HireFailure::NoVacancy(role))));
    assert_eq!(row.computer, npc.computer);
    assert_eq!(row.job_role, role);
    assert!(account.is_some());
    assert_eq!(recently, Some(Err(FireFailure::HiredRecently)));
    assert_eq!(later, Some(Ok(())));
}

#[tokio::test]
async fn should_make_a_hired_admin_the_networks_admin() {
    // Arrange
    let (world, network) = world_with(NetworkKind::Supermarket).await;
    let address = public_ip(&world, network).await;
    let admin = world
        .modules
        .npcs
        .npcs
        .network_admin_person(network)
        .await
        .unwrap()
        .unwrap();
    let user = world
        .modules
        .npcs
        .npcs
        .npc(admin)
        .await
        .unwrap()
        .unwrap()
        .user;

    // Act
    world
        .modules
        .npcs
        .jobs
        .fire_npc(&address, &user)
        .await
        .unwrap();

    let hired = world
        .modules
        .npcs
        .jobs
        .hire_npc(&address, JobRole::Admin)
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    // Assert
    assert_eq!(
        world
            .modules
            .npcs
            .npcs
            .network_admin_person(network)
            .await
            .unwrap(),
        Some(hired)
    );
}
