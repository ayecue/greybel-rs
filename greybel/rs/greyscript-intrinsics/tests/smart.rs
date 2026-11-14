use greybel_test_support::future::block_on;
use greyhack_world::World;

use crate::support::save_copy;
use crate::support::scripts::run_on_host;
use crate::support::smart::world_with_appliance;

#[test]
fn should_expose_and_change_smart_appliance_settings() {
    let world = world_with_appliance();
    let code = r#"
device = include_lib("/lib/libsmartappliance.so")
print device.model
print device.set_alarm(1)
print device.override_settings(100, 50)
print device.override_settings(600, 20)
print device.set_alarm(0)
print device.model
"#;

    let (output, error, _) = run_on_host(code, world, true);

    assert_eq!(error, None);
    assert_eq!(
        output,
        "ACME/FR1\n1\noverride_settings: invalid power or temperature for this model. Check the device manual.\n\
         override_settings: warning: Unsafe settings applied. The safety alarm will shut down the appliance if these \
         values are reached.\n1\nerror: Device hardware malfunction"
    );
}

#[test]
fn should_deny_locked_smart_appliance_settings() {
    let world = world_with_appliance();
    let player = world.handle.player();
    let mut settings = block_on(world.modules.smart_devices.appliances.smart(player))
        .unwrap()
        .unwrap();

    settings.override_enabled = false;
    block_on(
        world
            .modules
            .smart_devices
            .appliances
            .save_smart(player, Some(&settings)),
    )
    .unwrap();

    let code = r#"
device = include_lib("/lib/libsmartappliance.so")
print device.override_settings(600, 20)
print device.set_alarm(1)
"#;

    let (output, error, _) = run_on_host(code, world, true);

    assert_eq!(error, None);
    assert!(output.contains("override_settings: denied"));
    assert!(output.contains("set_alarm: denied"));
}

#[tokio::test]
async fn should_preserve_smart_appliance_settings() {
    let mut world = world_with_appliance();
    let player = world.handle.player();
    let mut settings = world
        .modules
        .smart_devices
        .appliances
        .smart(player)
        .await
        .unwrap()
        .unwrap();

    settings.alarm = true;
    world
        .modules
        .smart_devices
        .appliances
        .save_smart(player, Some(&settings))
        .await
        .unwrap();

    let database = save_copy(&mut world).await;

    let restored = World::load(&database).await.unwrap();

    let settings = restored
        .modules
        .smart_devices
        .appliances
        .smart(restored.player())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(settings.model, "ACME/FR1");
    assert!(settings.alarm);
    assert_eq!(settings.required_power, 600);
}
