use greyhack_behaviour::{DeviceKind, ROOT_USER};

use crate::{ComputerId, SmartApplianceSettings, World, WorldOptions};

pub async fn appliance_and_camera() -> (World, ComputerId, ComputerId) {
    let mut world = World::generate(&WorldOptions {
        networks: 1,
        ..WorldOptions::default()
    })
    .await
    .unwrap();
    let mut devices = Vec::new();

    for address in world.public_addresses().await.unwrap() {
        let router = world.router_at(&address).await.unwrap().unwrap();
        let network_devices = world.devices(router.id).await.unwrap();

        if network_devices.len() >= 2 {
            devices = network_devices;
            break;
        }
    }

    let camera = devices[0];
    let appliance = devices[1];
    let mut camera_row = world.computer(camera).await.unwrap().unwrap();

    camera_row.kind = DeviceKind::Camera;
    world.save_computer(&camera_row).await.unwrap();
    world.set_password(camera, ROOT_USER, "pw").await.unwrap();
    world
        .save_smart(
            appliance,
            Some(&SmartApplianceSettings {
                override_enabled: true,
                ..SmartApplianceSettings::new("fridge", 600, 20)
            }),
        )
        .await
        .unwrap();

    (world, appliance, camera)
}
