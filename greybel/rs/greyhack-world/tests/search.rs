use greyhack_world::World;
use greyhack_world_creation::NetworkOptions;

use crate::support::search::id;
use crate::support::{
    change_router, computer, computers, router, routers, save_copy, world_options,
};

/// Tests that indexed searches find the world's routers, devices and credentials.
#[tokio::test]
async fn should_find_the_worlds_entities_by_their_search_keys() {
    let world = World::generate(&world_options(11, 6)).await.unwrap();
    let all_routers = routers(&world).await;

    for router in all_routers.iter().filter(|router| router.is_main()) {
        let found = world
            .modules
            .core
            .networks
            .routers
            .router_at(&router.public_ip)
            .await
            .unwrap();
        let domain = router.domain.to_uppercase();
        let bssid = router.bssid.clone().unwrap_or_default().to_lowercase();

        assert_eq!(id(found), Some(router.id));

        if !router.domain.is_empty() {
            assert_eq!(
                world
                    .modules
                    .core
                    .networks
                    .routers
                    .lookup(&domain)
                    .await
                    .unwrap(),
                Some(router.public_ip.clone())
            );
        }

        assert_eq!(
            id(world
                .modules
                .core
                .networks
                .routers
                .router_by_wifi(&bssid, router.essid.as_deref().unwrap_or_default())
                .await
                .unwrap()),
            Some(router.id)
        );

        let mut expected: Vec<_> = all_routers
            .iter()
            .filter(|other| other.is_main())
            .filter(|other| {
                let x = router.map[0] - other.map[0];
                let y = router.map[1] - other.map[1];

                (x * x + y * y).sqrt() < 0.5
            })
            .map(|router| router.id)
            .collect();
        let mut found: Vec<_> = world
            .modules
            .core
            .networks
            .routers
            .routers_in_range(router.map)
            .await
            .unwrap()
            .into_iter()
            .map(|router| router.id)
            .collect();

        expected.sort();
        found.sort();
        assert_eq!(found, expected);
    }

    for computer in computers(&world).await {
        for user in world
            .modules
            .core
            .computers
            .users
            .users(computer.id)
            .await
            .unwrap()
        {
            assert_eq!(
                world
                    .modules
                    .core
                    .computers
                    .users
                    .password_of_hash(&user.password_hash)
                    .await
                    .unwrap(),
                Some(user.password)
            );
        }

        assert_eq!(
            world
                .modules
                .core
                .networks
                .topology
                .device(computer.network, &computer.local_ip)
                .await
                .unwrap(),
            Some(computer.id)
        );
    }

    assert_eq!(
        world
            .modules
            .core
            .networks
            .routers
            .public_addresses()
            .await
            .unwrap()
            .len(),
        all_routers.iter().filter(|router| router.is_main()).count()
    );
    assert!(
        world
            .modules
            .core
            .networks
            .routers
            .router_at("1.2.3.4")
            .await
            .unwrap()
            .is_none()
    );
}

/// Saved changes are immediately searchable and remain searchable after a database reload.
#[tokio::test]
async fn should_find_saved_changes_and_preserve_them_across_a_reload() {
    let mut world = World::generate(&world_options(11, 6)).await.unwrap();
    let all_routers = routers(&world).await;
    let id_of_third = all_routers[2].id;
    let old_address = all_routers[2].public_ip.clone();

    change_router(&mut world, id_of_third, |router| {
        router.public_ip = "9.9.9.9".to_owned();
    })
    .await;

    assert_eq!(
        id(world
            .modules
            .core
            .networks
            .routers
            .router_at("9.9.9.9")
            .await
            .unwrap()),
        Some(id_of_third)
    );
    assert!(
        world
            .modules
            .core
            .networks
            .routers
            .router_at(&old_address)
            .await
            .unwrap()
            .is_none()
    );

    let added = world
        .modules
        .creation
        .network_generation
        .add_network(&NetworkOptions {
            public_ip: Some("8.8.4.4".to_owned()),
            domain: Some("added.com".to_owned()),
            ..NetworkOptions::default()
        })
        .await
        .unwrap();

    assert_eq!(
        world
            .modules
            .core
            .networks
            .routers
            .lookup("ADDED.com")
            .await
            .unwrap()
            .as_deref(),
        Some("8.8.4.4")
    );
    assert!(
        world
            .modules
            .core
            .networks
            .routers
            .public_addresses()
            .await
            .unwrap()
            .contains(&"8.8.4.4".to_owned())
    );

    let removed = all_routers[3].public_ip.clone();

    assert!(
        world
            .modules
            .economy
            .bank
            .remove_network(&removed)
            .await
            .unwrap()
    );
    assert!(
        world
            .modules
            .core
            .networks
            .routers
            .router_at(&removed)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        !world
            .modules
            .core
            .networks
            .routers
            .public_addresses()
            .await
            .unwrap()
            .contains(&removed)
    );

    let player = world.handle.player();

    world
        .modules
        .core
        .computers
        .users
        .set_password(player, "root", "changed")
        .await
        .unwrap();

    let hash = world
        .modules
        .core
        .computers
        .users
        .user(player, "root")
        .await
        .unwrap()
        .expect("root")
        .password_hash;

    assert_eq!(
        world
            .modules
            .core
            .computers
            .users
            .password_of_hash(&hash)
            .await
            .unwrap()
            .as_deref(),
        Some("changed")
    );

    let database = save_copy(&mut world).await;
    let world = World::load(&database).await.unwrap();

    assert_eq!(
        id(world
            .modules
            .core
            .networks
            .routers
            .router_at("9.9.9.9")
            .await
            .unwrap()),
        Some(id_of_third)
    );
    assert_eq!(
        id(world
            .modules
            .core
            .networks
            .routers
            .router_at("8.8.4.4")
            .await
            .unwrap()),
        Some(added)
    );
    assert!(
        world
            .modules
            .core
            .networks
            .routers
            .router_at(&removed)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        world
            .modules
            .core
            .computers
            .users
            .password_of_hash(&hash)
            .await
            .unwrap()
            .as_deref(),
        Some("changed")
    );
    assert_eq!(router(&world, added).await.public_ip, "8.8.4.4");
    assert_eq!(computer(&world, player).await.id, player);
}
