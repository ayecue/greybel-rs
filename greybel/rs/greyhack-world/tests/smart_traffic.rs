use greyhack_behaviour::{NetworkKind, license_plate};
use greyhack_world::{JobRole, TrafficFailure};

use crate::support::npcs_jobs::{public_ip, world_with};
use crate::support::smart_traffic::commute;

#[tokio::test]
async fn should_locate_a_driving_npcs_car_and_kill_them_in_an_accident() {
    // Arrange
    let (world, police) = world_with(NetworkKind::Police).await;
    let credentials = world
        .modules
        .smart_devices
        .traffic
        .traffic_credentials(police)
        .await
        .unwrap()
        .unwrap();
    let password = world
        .modules
        .core
        .computers
        .user(credentials.computer, &credentials.user)
        .await
        .unwrap()
        .unwrap()
        .password;
    let driver = world
        .modules
        .npcs
        .npcs
        .npcs_in(police)
        .await
        .unwrap()
        .remove(0);
    let secret = world.handle.world_seed().clone();
    let address = public_ip(&world, police).await;
    let plate = license_plate(&secret, &address, &driver.user).unwrap();

    world
        .modules
        .npcs
        .jobs
        .set_npc_job(driver.person, driver.job_role, Some(plate.clone()))
        .await
        .unwrap();

    let driver = world
        .modules
        .npcs
        .npcs
        .npc(driver.person)
        .await
        .unwrap()
        .unwrap();
    let mut traffic = world
        .modules
        .smart_devices
        .traffic
        .traffic(police)
        .await
        .unwrap();

    traffic.override_enabled = true;
    world
        .modules
        .smart_devices
        .traffic
        .save_traffic(police, &traffic)
        .await
        .unwrap();

    // Act
    world.handle.world_mut().time_seconds = commute(&world, &driver, false);

    let parked = world
        .modules
        .smart_devices
        .traffic
        .locate_vehicle(police, &plate, &password, true)
        .await
        .unwrap();
    let wrong = world
        .modules
        .smart_devices
        .traffic
        .locate_vehicle(police, &plate, "wrong", true)
        .await
        .unwrap();
    let invalid = world
        .modules
        .smart_devices
        .traffic
        .locate_vehicle(police, "nope", &password, true)
        .await
        .unwrap();
    let secret = world.handle.world_seed().clone();
    let unknown = world
        .modules
        .smart_devices
        .traffic
        .locate_vehicle(
            police,
            &license_plate(&secret, "1.2.3.4", "nobody").unwrap(),
            &password,
            true,
        )
        .await
        .unwrap();
    let no_car = world
        .modules
        .smart_devices
        .traffic
        .traffic_accident()
        .await
        .unwrap();

    world.handle.world_mut().time_seconds = commute(&world, &driver, true);

    let driving = world
        .modules
        .smart_devices
        .traffic
        .locate_vehicle(police, &plate, &password, true)
        .await
        .unwrap();
    let accident = world
        .modules
        .smart_devices
        .traffic
        .traffic_accident()
        .await
        .unwrap();
    let cooldown = world
        .modules
        .smart_devices
        .traffic
        .locate_vehicle(police, &plate, &password, true)
        .await
        .unwrap();
    let dead = world
        .modules
        .npcs
        .npcs
        .npc(driver.person)
        .await
        .unwrap()
        .unwrap();

    // Assert
    assert_eq!(parked, Err(TrafficFailure::OutOfSight));
    assert_eq!(wrong, Err(TrafficFailure::IncorrectPassword));
    assert_eq!(invalid, Err(TrafficFailure::InvalidPlate));
    assert_eq!(unknown, Err(TrafficFailure::UnknownPlate));
    assert!(!no_car);
    assert_eq!(driving, Ok(()));
    assert!(accident);
    assert_eq!(cooldown, Err(TrafficFailure::Cooldown));
    assert!(dead.is_dead(world.handle.time_seconds() + 6 * 60));
    assert!(
        world
            .modules
            .smart_devices
            .traffic
            .traffic(police)
            .await
            .unwrap()
            .override_used
    );
    assert_ne!(credentials.job_role, JobRole::None);
}
