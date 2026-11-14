use greyhack_behaviour::{FileData, PermissionSet, ServiceId};
use greyhack_world::{Login, NodeDraft, World};

use crate::support::world::snapshot;
use crate::support::{
    change_computer, change_router, computer, has_computer, router, routers, save_copy,
    world_options,
};

#[tokio::test]
async fn should_build_the_same_world_from_the_same_seed() {
    let options = world_options(42, 4);
    let first = World::generate(&options).await.unwrap();
    let second = World::generate(&options).await.unwrap();

    assert_eq!(snapshot(&first).await, snapshot(&second).await);
}

#[tokio::test]
async fn should_build_different_worlds_from_different_seeds() {
    let first = World::generate(&world_options(42, 4)).await.unwrap();
    let second = World::generate(&world_options(43, 4)).await.unwrap();

    assert_ne!(snapshot(&first).await, snapshot(&second).await);
}

#[tokio::test]
async fn should_report_the_kernel_library_version_of_every_router() {
    let world = World::generate(&world_options(42, 4)).await.unwrap();

    for router in routers(&world).await {
        let kernel = world
            .modules
            .core
            .networks
            .routers
            .kernel_library(router.id)
            .await
            .unwrap()
            .expect("a kernel library");

        assert_eq!(
            world
                .modules
                .core
                .networks
                .routers
                .kernel_version(router.id)
                .await
                .unwrap(),
            Some(kernel.version)
        );
    }
}

#[tokio::test]
async fn should_restore_the_whole_world_from_a_database() {
    let mut world = World::generate(&world_options(7, 4)).await.unwrap();
    let expected = snapshot(&world).await;
    let db = save_copy(&mut world).await;
    let restored = World::load(&db).await.unwrap();

    assert_eq!(snapshot(&restored).await, expected);
}

#[tokio::test]
async fn should_restore_the_random_state_from_a_database() {
    let mut world = World::generate(&world_options(7, 4)).await.unwrap();
    let db = save_copy(&mut world).await;
    let restored = World::load(&db).await.unwrap();
    let a = world
        .modules
        .creation
        .network_generation
        .add_network(&greyhack_world::NetworkOptions::default())
        .await
        .unwrap();
    let b = restored
        .modules
        .creation
        .network_generation
        .add_network(&greyhack_world::NetworkOptions::default())
        .await
        .unwrap();

    assert_eq!(router(&world, a).await, router(&restored, b).await);
}

#[tokio::test]
async fn should_find_a_persisted_wallet_by_user_with_its_balance_and_login() {
    let mut world = World::new(&greyhack_world::PlayerOptions::default())
        .await
        .unwrap();
    let wallet = world
        .modules
        .economy
        .wallets
        .add_wallet("alice", "pw", "0000", "test")
        .await
        .unwrap();

    world
        .modules
        .economy
        .wallets
        .set_balance(wallet, "Ether", 5.0)
        .await
        .unwrap();

    let database = save_copy(&mut world).await;
    let world = World::load(&database).await.unwrap();
    let found = world
        .modules
        .economy
        .wallets
        .wallet_by_user("alice")
        .await
        .unwrap()
        .unwrap();

    assert_eq!(found.id, wallet);
    assert_eq!(
        world
            .modules
            .economy
            .wallets
            .balance(wallet, "Ether")
            .await
            .unwrap(),
        Some(5.0)
    );
    assert_eq!(
        world
            .modules
            .economy
            .wallets
            .login_wallet("alice", "pw")
            .await
            .unwrap(),
        Some(wallet)
    );
}

#[tokio::test]
async fn should_give_a_new_wallet_an_id_of_its_own_after_a_reload() {
    let mut world = World::new(&greyhack_world::PlayerOptions::default())
        .await
        .unwrap();
    let first = world
        .modules
        .economy
        .wallets
        .add_wallet("alice", "pw", "0000", "test")
        .await
        .unwrap();
    let database = save_copy(&mut world).await;
    let world = World::load(&database).await.unwrap();

    let second = world
        .modules
        .economy
        .wallets
        .add_wallet("bob", "pw", "0000", "test")
        .await
        .unwrap();

    assert_ne!(first, second);
}

#[tokio::test]
async fn should_delete_a_coin_from_persisted_wallet_balances() {
    let mut world = World::new(&greyhack_world::PlayerOptions::default())
        .await
        .unwrap();
    let wallet = world
        .modules
        .economy
        .wallets
        .add_wallet("alice", "pw", "0000", "test")
        .await
        .unwrap();

    world
        .modules
        .economy
        .wallets
        .set_balance(wallet, "Ether", 5.0)
        .await
        .unwrap();
    world
        .modules
        .economy
        .wallets
        .set_balance(wallet, "Bitcoin", 2.0)
        .await
        .unwrap();

    let database = save_copy(&mut world).await;
    let mut world = World::load(&database).await.unwrap();
    let deleted = world
        .modules
        .economy
        .mine_jobs
        .delete_coin("Ether")
        .await
        .unwrap();
    let database = save_copy(&mut world).await;
    let world = World::load(&database).await.unwrap();

    assert!(deleted);
    assert_eq!(
        world
            .modules
            .economy
            .wallets
            .balance(wallet, "Ether")
            .await
            .unwrap(),
        None
    );
    assert_eq!(
        world
            .modules
            .economy
            .wallets
            .balance(wallet, "Bitcoin")
            .await
            .unwrap(),
        Some(2.0)
    );
}

#[tokio::test]
async fn should_give_every_network_a_unique_public_address() {
    let world = World::generate(&world_options(1, 50)).await.unwrap();
    let mut public: Vec<String> = routers(&world)
        .await
        .into_iter()
        .map(|router| router.public_ip)
        .collect();
    let count = public.len();

    public.sort_unstable();
    public.dedup();
    assert_eq!(public.len(), count);
    assert_eq!(count, 51);
}

#[tokio::test]
async fn should_give_every_device_a_unique_lan_address() {
    let world = World::generate(&world_options(1, 50)).await.unwrap();

    for router in routers(&world).await {
        let mut lan = Vec::new();

        for id in world
            .modules
            .core
            .networks
            .topology
            .devices(router.id)
            .await
            .unwrap()
        {
            lan.push(computer(&world, id).await.local_ip);
        }

        let devices = lan.len();

        lan.sort_unstable();
        lan.dedup();
        assert_eq!(lan.len(), devices);
        assert!(!lan.contains(&router.local_ip));
    }
}

#[tokio::test]
async fn should_remove_a_network_with_its_computers_and_database_rows() {
    let mut world = World::generate(&world_options(3, 4)).await.unwrap();
    let router = routers(&world).await.swap_remove(1);
    let devices = world
        .modules
        .core
        .networks
        .topology
        .devices(router.id)
        .await
        .unwrap();
    let removed = world
        .modules
        .economy
        .bank
        .remove_network(&router.public_ip)
        .await
        .unwrap();

    assert!(removed);
    assert!(
        world
            .modules
            .core
            .networks
            .routers
            .router_at(&router.public_ip)
            .await
            .unwrap()
            .is_none()
    );

    for id in &devices {
        assert!(!has_computer(&world, *id).await);
    }

    let database = save_copy(&mut world).await;
    let restored = World::load(&database).await.unwrap();

    assert!(
        restored
            .modules
            .core
            .networks
            .routers
            .router_at(&router.public_ip)
            .await
            .unwrap()
            .is_none()
    );

    for id in devices {
        assert!(!has_computer(&restored, id).await);
        assert_eq!(
            restored
                .modules
                .core
                .files
                .sizes
                .file_count(id)
                .await
                .unwrap(),
            0
        );
    }
}

#[tokio::test]
async fn should_not_remove_the_network_of_the_player() {
    let world = World::generate(&world_options(3, 4)).await.unwrap();
    let router = routers(&world).await.swap_remove(1);

    world
        .modules
        .economy
        .bank
        .remove_network(&router.public_ip)
        .await
        .unwrap();

    let player_ip = world
        .modules
        .core
        .networks
        .topology
        .public_ip(world.handle.player())
        .await
        .unwrap();
    let removed = world
        .modules
        .economy
        .bank
        .remove_network(&player_ip)
        .await
        .unwrap();

    assert!(!removed);
}

#[tokio::test]
async fn should_persist_edits_across_a_database_reload() {
    let mut world = World::new(&greyhack_world::PlayerOptions::default())
        .await
        .unwrap();
    let player = world.handle.player();
    let home = world
        .modules
        .core
        .files
        .lookup
        .folder(player, "/home", true)
        .await
        .unwrap()
        .unwrap();
    let notes = world
        .modules
        .libraries
        .generator
        .add_full_node(
            &home,
            &NodeDraft::folder("notes", "root", PermissionSet::default_for(true)),
        )
        .await
        .unwrap();
    let mut todo = world
        .modules
        .libraries
        .generator
        .add_full_node(
            &notes,
            &NodeDraft::file(
                "todo.txt",
                "root",
                PermissionSet::default_for(false),
                FileData::text("first"),
            ),
        )
        .await
        .unwrap();

    todo.data_mut().unwrap().content = "second".to_owned();
    world
        .modules
        .core
        .files
        .changes
        .save_node(&todo)
        .await
        .unwrap();
    change_computer(&mut world, player, |computer| {
        computer.name = "renamed".to_owned();
    })
    .await;
    world
        .modules
        .core
        .computers
        .users
        .add_user(player, "guest2", "password")
        .await
        .unwrap()
        .unwrap();

    let router_id = computer(&world, player).await.router.unwrap();

    change_router(&mut world, router_id, |router| {
        router.firewall_active = false;
    })
    .await;
    world
        .modules
        .economy
        .bank
        .add_bank("12345678", "password", 50.0)
        .await
        .unwrap();
    world
        .modules
        .npcs
        .mail
        .add_mail_account("new@mail.com", "password")
        .await
        .unwrap();
    world
        .modules
        .npcs
        .mail
        .deliver_mail("new@mail.com", "someone@mail.com", "Hello", "A mail.")
        .await
        .unwrap();
    world
        .modules
        .services
        .installs
        .install_service(player, ServiceId::Ssh, "1.0.0")
        .await
        .unwrap();

    let database = save_copy(&mut world).await;
    let restored = World::load(&database).await.unwrap();

    assert_eq!(computer(&restored, player).await.name, "renamed");
    assert_eq!(
        restored
            .modules
            .core
            .files
            .lookup
            .content(player, "/home/notes/todo.txt", true)
            .await
            .unwrap(),
        Some("second".to_owned())
    );
    assert!(
        restored
            .modules
            .core
            .computers
            .users
            .check_password(player, "guest2", "password")
            .await
            .unwrap()
    );
    assert!(!router(&restored, router_id).await.firewall_active);
    assert_eq!(
        restored
            .modules
            .economy
            .bank
            .bank_account("12345678")
            .await
            .unwrap()
            .unwrap()
            .money,
        50.0
    );
    assert_eq!(
        restored
            .modules
            .npcs
            .mail
            .mail_login("new@mail.com", "password")
            .await
            .unwrap(),
        Login::Valid
    );
    assert!(
        restored
            .modules
            .core
            .computers
            .processes
            .has_service(player, ServiceId::Ssh)
            .await
            .unwrap()
    );
}
