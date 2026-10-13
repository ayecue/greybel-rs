use greyhack_behaviour::{LibraryKind, hash_password};
use greyhack_world::{Library, NetCard, PlayerOptions, World};

use crate::support::save_copy;

#[tokio::test]
async fn should_install_a_network_card_and_persist_it() {
    let mut world = World::new(&PlayerOptions::default()).await.unwrap();
    let player = world.player();
    let count = world.net_cards(player).await.unwrap().len();
    let card = NetCard::new(true, true);

    assert!(world.add_net_card(player, &card).await.unwrap());

    let cards = world.net_cards(player).await.unwrap();

    assert_eq!(cards.len(), count + 1);
    assert_eq!(cards.last(), Some(&card));

    let database = save_copy(&mut world).await;
    let restored = World::load(&database).await.unwrap();

    assert_eq!(
        restored.net_cards(player).await.unwrap().last(),
        Some(&card)
    );
}

#[tokio::test]
async fn should_change_library_file_and_port_versions() {
    let mut world = World::new(&PlayerOptions::default()).await.unwrap();
    let player = world.player();
    let file = world
        .file(player, "/lib/libssh.so", false)
        .await
        .unwrap()
        .unwrap();

    world
        .set_library_of(
            &file,
            Some(&Library {
                kind: LibraryKind::Ssh,
                version: "1.0.0".to_owned(),
                zones: Vec::new(),
            }),
        )
        .await
        .unwrap();
    world
        .install_service(player, greyhack_behaviour::ServiceId::Ssh, "1.0.0")
        .await
        .unwrap();

    assert!(
        world
            .set_library_version(player, "/lib/libssh.so", "9.8.7")
            .await
            .unwrap()
    );
    assert!(
        world
            .set_port_library_version(player, 22, "8.7.6")
            .await
            .unwrap()
    );

    let file = world
        .file(player, "/lib/libssh.so", false)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        world
            .library_of(&file)
            .await
            .unwrap()
            .map(|library| library.version),
        Some("9.8.7".to_owned())
    );
    assert_eq!(
        world
            .port(player, 22)
            .await
            .unwrap()
            .map(|port| port.version),
        Some("8.7.6".to_owned())
    );
    assert!(
        !world
            .set_library_version(player, "/lib/missing.so", "1.0.0")
            .await
            .unwrap()
    );
    assert!(
        !world
            .set_port_library_version(player, 65535, "1.0.0")
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn should_register_a_password_without_an_account() {
    let mut world = World::new(&PlayerOptions::default()).await.unwrap();
    let hash = world.register_password("standalone").await.unwrap();

    assert_eq!(hash, hash_password("standalone"));
    assert_eq!(
        world.password_of_hash(&hash).await.unwrap(),
        Some("standalone".to_owned())
    );

    let database = save_copy(&mut world).await;
    let restored = World::load(&database).await.unwrap();

    assert_eq!(
        restored.password_of_hash(&hash).await.unwrap(),
        Some("standalone".to_owned())
    );
}
