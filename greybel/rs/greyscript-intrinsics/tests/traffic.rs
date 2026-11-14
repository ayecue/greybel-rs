use greybel_test_support::future::block_on;
use greyhack_behaviour::{NetworkKind, license_plate};
use greyhack_world::World;

use crate::support::save_copy;
use crate::support::scripts::run_on_host;
use crate::support::traffic::{TrafficWorld, enable_override, world_with_traffic};

#[test]
fn should_expose_traffic_credentials_and_locate_a_vehicle() {
    let TrafficWorld { world, plate, .. } = world_with_traffic();
    let unknown = license_plate(&world.handle.world_seed(), "1.2.3.4", "nobody").unwrap();
    let code = format!(
        "device = include_lib(\"/lib/libtrafficnet.so\")\nprint device.get_credentials_info\nprint device.locate_vehicle(\"{plate}\", \"secret\")\nprint device.locate_vehicle(\"{plate}\", \"bad\")\nprint device.locate_vehicle(\"nope\", \"secret\")\nprint device.locate_vehicle(\"{unknown}\", \"secret\")"
    );

    let (output, error, _) = run_on_host(&code, world, true);

    assert_eq!(error, None);
    assert_eq!(
        output,
        "Commissioner Alice alice\n1\nerror: Incorrect password. Access denied.\nerror: Invalid license plate.\nlocate_vehicle: License plate not found."
    );
}

#[test]
fn should_throttle_camera_calls() {
    let TrafficWorld { world, .. } = world_with_traffic();
    let code = r#"
device = include_lib("/lib/libtrafficnet.so")
print device.camera_link_system
print device.camera_link_system
"#;

    let (output, error, _) = run_on_host(code, world, true);

    assert_eq!(error, None);
    assert!(output.starts_with("1\nError: Unable to use the camera at this time. Too many calls."));
}

#[tokio::test]
async fn should_preserve_traffic_network_state() {
    let TrafficWorld {
        mut world,
        network,
        driver,
        ..
    } = world_with_traffic();
    let database = save_copy(&mut world).await;

    let restored = World::load(&database).await.unwrap();

    assert_eq!(
        restored
            .modules
            .core
            .networks
            .settings
            .network(network)
            .await
            .unwrap()
            .unwrap()
            .kind,
        NetworkKind::Police
    );
    assert_eq!(
        restored
            .modules
            .smart_devices
            .traffic
            .traffic_credentials(network)
            .await
            .unwrap()
            .unwrap()
            .name,
        "Alice alice"
    );
    assert!(
        restored
            .modules
            .npcs
            .npcs
            .npc(driver)
            .await
            .unwrap()
            .unwrap()
            .license_plate
            .is_some()
    );
}

#[test]
fn should_kill_the_driver_of_a_located_car_and_block_the_cameras_for_24_hours() {
    let TrafficWorld {
        mut world,
        network,
        plate,
        driver,
    } = world_with_traffic();

    enable_override(&mut world, network);

    let code = format!(
        r#"
device = include_lib("/lib/libtrafficnet.so")
admin = include_lib("admin.so")
print admin.traffic_accident
print device.locate_vehicle("{plate}", "secret")
print admin.traffic_accident
print device.camera_link_system
print device.locate_vehicle("{plate}", "secret")
"#
    );

    let (output, error, restored) = run_on_host(&code, world, true);

    assert_eq!(error, None);
    assert_eq!(
        output,
        "0\n1\n1\nError: This user cannot access the global camera system for another 12 hours.\nError: This user cannot access the global camera system for another 12 hours."
    );
    assert!(
        restored
            .modules
            .smart_devices
            .traffic
            .has_traffic_cooldown()
    );
    assert!(
        block_on(restored.modules.smart_devices.traffic.traffic(network))
            .unwrap()
            .override_used
    );
    assert!(
        block_on(restored.modules.npcs.npcs.npc(driver))
            .unwrap()
            .unwrap()
            .dead_at
            .is_some()
    );
}

#[test]
fn should_expire_the_traffic_cooldown_after_twelve_hours() {
    let TrafficWorld {
        mut world,
        network,
        plate,
        ..
    } = world_with_traffic();

    enable_override(&mut world, network);
    block_on(
        world
            .modules
            .smart_devices
            .traffic
            .locate_vehicle(network, &plate, "secret", true),
    )
    .unwrap()
    .unwrap();

    assert!(block_on(world.modules.smart_devices.traffic.traffic_accident()).unwrap());
    assert!(!block_on(world.modules.smart_devices.traffic.traffic_accident()).unwrap());
    block_on(world.tick(43_199)).unwrap();
    assert!(world.modules.smart_devices.traffic.has_traffic_cooldown());
    block_on(world.tick(1)).unwrap();
    assert!(!world.modules.smart_devices.traffic.has_traffic_cooldown());
}

#[tokio::test]
async fn should_preserve_the_traffic_cooldown_and_override_flag() {
    let TrafficWorld {
        mut world,
        network,
        plate,
        ..
    } = world_with_traffic();

    enable_override(&mut world, network);
    world
        .modules
        .smart_devices
        .traffic
        .locate_vehicle(network, &plate, "secret", true)
        .await
        .unwrap()
        .unwrap();

    assert!(
        world
            .modules
            .smart_devices
            .traffic
            .traffic_accident()
            .await
            .unwrap()
    );

    let database = save_copy(&mut world).await;

    let restored = World::load(&database).await.unwrap();

    assert!(
        restored
            .modules
            .smart_devices
            .traffic
            .has_traffic_cooldown()
    );

    let traffic = restored
        .modules
        .smart_devices
        .traffic
        .traffic(network)
        .await
        .unwrap();

    assert!(traffic.override_enabled);
    assert!(traffic.override_used);
}
