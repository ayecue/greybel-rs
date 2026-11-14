use greybel_test_support::future::block_on;
use greyhack_behaviour::DeviceKind;
use greyhack_world::{ComputerOptions, PlayerOptions, World};

use crate::support;
use crate::support::read_computer;

#[test]
fn should_return_a_router_only_for_a_switch_device() {
    let world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let router = read_computer(&world, world.handle.player()).router.unwrap();
    let switch = block_on(world.modules.creation.network_generation.add_computer_to(
        router,
        &ComputerOptions {
            lan_ip: Some("192.168.0.50".to_owned()),
            is_switch: true,
            ..ComputerOptions::default()
        },
    ))
    .unwrap()
    .expect("the switch is created");

    block_on(world.modules.creation.network_generation.add_computer_to(
        router,
        &ComputerOptions {
            lan_ip: Some("192.168.0.51".to_owned()),
            ..ComputerOptions::default()
        },
    ))
    .unwrap()
    .expect("the ordinary computer is created");
    assert_eq!(read_computer(&world, switch).kind, DeviceKind::Switch);

    let (output, error, _) = support::scripts::run_on_host(
        "print(typeof(get_switch(\"192.168.0.50\")))\nprint(get_switch(\"192.168.0.51\"))",
        world,
        false,
    );

    assert_eq!(error, None);
    assert_eq!(output, "router\nnull");
}

#[test]
fn should_persist_the_switch_marker() {
    block_on(async {
        let mut world = World::new(&PlayerOptions::default()).await.unwrap();
        let router = read_computer(&world, world.handle.player()).router.unwrap();
        let switch = world
            .modules
            .creation
            .network_generation
            .add_computer_to(
                router,
                &ComputerOptions {
                    lan_ip: Some("192.168.0.50".to_owned()),
                    is_switch: true,
                    ..ComputerOptions::default()
                },
            )
            .await
            .unwrap()
            .unwrap();
        let database = support::save_copy(&mut world).await;
        let loaded = World::load(&database).await.expect("the world loads");

        assert_eq!(read_computer(&loaded, switch).kind, DeviceKind::Switch);
    });
}

#[test]
fn should_reject_a_switch_lookup_outside_the_lan() {
    let world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let (output, error, _) = support::scripts::run_on_host(
        "print(get_switch(\"8.8.8.8\"))\nprint(get_switch(\"192.168.0.99\"))",
        world,
        false,
    );

    assert_eq!(error, None);
    assert_eq!(output, "null\nnull");
}
