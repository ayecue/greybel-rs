use greyhack_behaviour::{NetworkKind, license_plate};

use crate::systems::npcs::jobs::tests::support::{public_ip, world_with};
use crate::{JobRole, TrafficFailure};

#[path = "support/traffic.rs"]
mod support;
use support::commute;

#[tokio::test]
async fn should_locate_a_driving_npcs_car_and_kill_them_in_an_accident() {
    // Arrange
    let (mut world, police) = world_with(NetworkKind::Police).await;
    let credentials = world.traffic_credentials(police).await.unwrap().unwrap();
    let password = world
        .db
        .user(credentials.computer, &credentials.user)
        .await
        .unwrap()
        .unwrap()
        .password;
    let driver = world.db.npcs_in(police).await.unwrap().remove(0);
    let plate = license_plate(
        world.world_seed(),
        &public_ip(&world, police).await,
        &driver.user,
    )
    .unwrap();

    world
        .set_npc_job(driver.person, driver.job_role, Some(plate.clone()))
        .await
        .unwrap();

    let driver = world.db.npc(driver.person).await.unwrap().unwrap();
    let mut traffic = world.traffic(police).await.unwrap();

    traffic.override_enabled = true;
    world.save_traffic(police, &traffic).await.unwrap();

    // Act
    world.time_seconds = commute(&world, &driver, false);

    let parked = world
        .locate_vehicle(police, &plate, &password, true)
        .await
        .unwrap();
    let wrong = world
        .locate_vehicle(police, &plate, "wrong", true)
        .await
        .unwrap();
    let invalid = world
        .locate_vehicle(police, "nope", &password, true)
        .await
        .unwrap();
    let unknown = world
        .locate_vehicle(
            police,
            &license_plate(world.world_seed(), "1.2.3.4", "nobody").unwrap(),
            &password,
            true,
        )
        .await
        .unwrap();
    let no_car = world.traffic_accident().await.unwrap();

    world.time_seconds = commute(&world, &driver, true);

    let driving = world
        .locate_vehicle(police, &plate, &password, true)
        .await
        .unwrap();
    let accident = world.traffic_accident().await.unwrap();
    let cooldown = world
        .locate_vehicle(police, &plate, &password, true)
        .await
        .unwrap();
    let dead = world.db.npc(driver.person).await.unwrap().unwrap();

    // Assert
    assert_eq!(parked, Err(TrafficFailure::OutOfSight));
    assert_eq!(wrong, Err(TrafficFailure::IncorrectPassword));
    assert_eq!(invalid, Err(TrafficFailure::InvalidPlate));
    assert_eq!(unknown, Err(TrafficFailure::UnknownPlate));
    assert!(!no_car);
    assert_eq!(driving, Ok(()));
    assert!(accident);
    assert_eq!(cooldown, Err(TrafficFailure::Cooldown));
    assert!(dead.is_dead(world.time_seconds + 6 * 60));
    assert!(world.traffic(police).await.unwrap().override_used);
    assert_ne!(credentials.job_role, JobRole::None);
}
