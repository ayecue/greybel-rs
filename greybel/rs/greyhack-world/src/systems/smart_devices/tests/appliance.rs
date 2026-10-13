use crate::ApplianceFailure;

#[path = "support/appliance.rs"]
mod support;
use support::appliance_and_camera;

#[tokio::test]
async fn should_refuse_override_values_other_than_the_models() {
    let (mut world, appliance, _) = appliance_and_camera().await;

    assert_eq!(
        world
            .override_appliance(appliance, 1000.0, 100.0)
            .await
            .unwrap(),
        Err(ApplianceFailure::InvalidSettings)
    );
    assert_eq!(
        world
            .override_appliance(appliance, 600.5, 20.0)
            .await
            .unwrap(),
        Err(ApplianceFailure::InvalidSettings)
    );
}

#[tokio::test]
async fn should_warn_while_the_alarm_is_on_then_break_once_it_is_off() {
    let (mut world, appliance, camera) = appliance_and_camera().await;

    assert_eq!(
        world
            .override_appliance(appliance, 600.0, 20.0)
            .await
            .unwrap(),
        Err(ApplianceFailure::AlarmWarning)
    );
    assert!(world.access_cctv(camera, Some("pw")).await.unwrap().is_ok());
    assert_eq!(
        world.set_appliance_alarm(appliance, false).await.unwrap(),
        Ok(())
    );

    let smart = world.smart(appliance).await.unwrap().unwrap();

    assert!(smart.is_broken(world.time_seconds()));
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
        world.access_cctv(camera, Some("pw")).await.unwrap(),
        Err(crate::CctvFailure::NoExternalAccess)
    );
    assert_eq!(
        world
            .override_appliance(appliance, 600.0, 20.0)
            .await
            .unwrap(),
        Err(ApplianceFailure::Malfunction)
    );
}

#[tokio::test]
async fn should_repair_the_appliance_a_day_after_the_accident() {
    let (mut world, appliance, camera) = appliance_and_camera().await;

    assert_eq!(
        world.set_appliance_alarm(appliance, false).await.unwrap(),
        Ok(())
    );
    assert_eq!(
        world
            .override_appliance(appliance, 600.0, 20.0)
            .await
            .unwrap(),
        Err(ApplianceFailure::Malfunction)
    );
    world.tick(24 * 60 * 60).await.unwrap();

    assert!(
        !world
            .smart(appliance)
            .await
            .unwrap()
            .unwrap()
            .is_broken(world.time_seconds())
    );
    assert!(world.access_cctv(camera, Some("pw")).await.unwrap().is_ok());
}
