use greyhack_behaviour::{DeviceKind, ROOT_USER};
use greyhack_world::{ComputerId, SmartApplianceSettings, World, WorldOptions};

pub async fn appliance_and_camera() -> (World, ComputerId, ComputerId) {
    let world = World::generate(&WorldOptions {
        networks: 1,
        ..WorldOptions::default()
    })
    .await
    .unwrap();
    let mut devices = Vec::new();

    for address in world
        .modules
        .core
        .networks
        .routers
        .public_addresses()
        .await
        .unwrap()
    {
        let router = world
            .modules
            .core
            .networks
            .routers
            .router_at(&address)
            .await
            .unwrap()
            .unwrap();
        let network_devices = world
            .modules
            .core
            .networks
            .topology
            .devices(router.id)
            .await
            .unwrap();

        if network_devices.len() >= 2 {
            devices = network_devices;
            break;
        }
    }

    let camera = devices[0];
    let appliance = devices[1];
    let mut camera_row = world
        .modules
        .core
        .networks
        .topology
        .computer(camera)
        .await
        .unwrap()
        .unwrap();

    camera_row.kind = DeviceKind::Camera;
    world
        .modules
        .core
        .networks
        .topology
        .save_computer(&camera_row)
        .await
        .unwrap();
    world
        .modules
        .core
        .computers
        .users
        .set_password(camera, ROOT_USER, "pw")
        .await
        .unwrap();
    world
        .modules
        .smart_devices
        .appliances
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
