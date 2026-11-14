use greyhack_world::{Flood, PlayerOptions, World};

use crate::support::flood::rebooting;

#[tokio::test]
async fn should_reboot_a_computer_once_enough_addresses_flood_it() {
    let world = World::new(&PlayerOptions::default()).await.unwrap();
    let target = world.handle.player();

    let first = world
        .modules
        .intrusion
        .floods
        .flood(target, "1.1.1.1", 2)
        .await
        .unwrap();

    assert!(matches!(first, Flood::Active(_)));
    assert!(!rebooting(&world).await);
    assert_eq!(
        world
            .modules
            .intrusion
            .floods
            .flood(target, "2.2.2.2", 2)
            .await
            .unwrap(),
        Flood::Reboot
    );
    assert!(rebooting(&world).await);
}

#[tokio::test]
async fn should_count_an_address_flooding_twice_once() {
    let world = World::new(&PlayerOptions::default()).await.unwrap();
    let target = world.handle.player();

    for _ in 0..2 {
        let flood = world
            .modules
            .intrusion
            .floods
            .flood(target, "1.1.1.1", 2)
            .await
            .unwrap();

        assert!(matches!(flood, Flood::Active(_)));
    }

    assert_eq!(
        world
            .modules
            .intrusion
            .floods
            .flood(target, "2.2.2.2", 2)
            .await
            .unwrap(),
        Flood::Reboot
    );
}

#[tokio::test]
async fn should_forget_a_flood_once_its_script_stops() {
    let world = World::new(&PlayerOptions::default()).await.unwrap();
    let target = world.handle.player();
    let Flood::Active(work) = world
        .modules
        .intrusion
        .floods
        .flood(target, "1.1.1.1", 2)
        .await
        .unwrap()
    else {
        panic!("the first flood waits");
    };

    assert!(world.modules.intrusion.floods.cancel(work).await.unwrap());

    let second = world
        .modules
        .intrusion
        .floods
        .flood(target, "2.2.2.2", 2)
        .await
        .unwrap();

    assert!(matches!(second, Flood::Active(_)));
    assert!(!rebooting(&world).await);
}

#[tokio::test]
async fn should_start_counting_again_after_a_reboot() {
    let world = World::new(&PlayerOptions::default()).await.unwrap();
    let target = world.handle.player();

    world
        .modules
        .intrusion
        .floods
        .flood(target, "1.1.1.1", 2)
        .await
        .unwrap();
    world
        .modules
        .intrusion
        .floods
        .flood(target, "2.2.2.2", 2)
        .await
        .unwrap();

    let after = world
        .modules
        .intrusion
        .floods
        .flood(target, "1.1.1.1", 2)
        .await
        .unwrap();

    assert!(matches!(after, Flood::Active(_)));
}
