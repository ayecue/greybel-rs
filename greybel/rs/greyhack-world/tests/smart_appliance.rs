use greyhack_world_smart_devices::ApplianceFailure;

use crate::support::smart_appliance::appliance_and_camera;

#[tokio::test]
async fn should_refuse_override_values_other_than_the_models() {
    let (world, appliance, _) = appliance_and_camera().await;

    assert_eq!(
        world
            .modules
            .smart_devices
            .appliances
            .override_appliance(appliance, 1000.0, 100.0)
            .await
            .unwrap(),
        Err(ApplianceFailure::InvalidSettings)
    );
    assert_eq!(
        world
            .modules
            .smart_devices
            .appliances
            .override_appliance(appliance, 600.5, 20.0)
            .await
            .unwrap(),
        Err(ApplianceFailure::InvalidSettings)
    );
}

#[tokio::test]
async fn should_warn_while_the_alarm_is_on_then_break_once_it_is_off() {
    let (world, appliance, camera) = appliance_and_camera().await;

    assert_eq!(
        world
            .modules
            .smart_devices
            .appliances
            .override_appliance(appliance, 600.0, 20.0)
            .await
            .unwrap(),
        Err(ApplianceFailure::AlarmWarning)
    );
    assert!(
        world
            .modules
            .smart_devices
            .cctv
            .access_cctv(camera, Some("pw"))
            .await
            .unwrap()
            .is_ok()
    );
    assert_eq!(
        world
            .modules
            .smart_devices
            .appliances
            .set_appliance_alarm(appliance, false)
            .await
            .unwrap(),
        Ok(())
    );

    let smart = world
        .modules
        .smart_devices
        .appliances
        .smart(appliance)
        .await
        .unwrap()
        .unwrap();

    assert!(smart.is_broken(world.handle.time_seconds()));
    assert_eq!(
        (
            smart.alarm,
            smart.power,
            smart.temperature,
            smart.override_enabled
        ),
        (true, 0, 0, false)
    );
    assert_eq!(
        world
            .modules
            .smart_devices
            .cctv
            .access_cctv(camera, Some("pw"))
            .await
            .unwrap(),
        Err(greyhack_world_smart_devices::CctvFailure::NoExternalAccess)
    );
    assert_eq!(
        world
            .modules
            .smart_devices
            .appliances
            .override_appliance(appliance, 600.0, 20.0)
            .await
            .unwrap(),
        Err(ApplianceFailure::Malfunction)
    );
}

#[tokio::test]
async fn should_repair_the_appliance_a_day_after_the_accident() {
    let (world, appliance, camera) = appliance_and_camera().await;

    assert_eq!(
        world
            .modules
            .smart_devices
            .appliances
            .set_appliance_alarm(appliance, false)
            .await
            .unwrap(),
        Ok(())
    );
    assert_eq!(
        world
            .modules
            .smart_devices
            .appliances
            .override_appliance(appliance, 600.0, 20.0)
            .await
            .unwrap(),
        Err(ApplianceFailure::Malfunction)
    );
    world.tick(24 * 60 * 60).await.unwrap();

    assert!(
        !world
            .modules
            .smart_devices
            .appliances
            .smart(appliance)
            .await
            .unwrap()
            .unwrap()
            .is_broken(world.handle.time_seconds())
    );
    assert!(
        world
            .modules
            .smart_devices
            .cctv
            .access_cctv(camera, Some("pw"))
            .await
            .unwrap()
            .is_ok()
    );
}
