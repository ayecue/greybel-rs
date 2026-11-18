use greyhack_behaviour::{FIRST_VERSION, FileData, LibraryKind, PermissionSet, ROOT_USER};
use greyhack_world::{World, WorldOptions};
use greyhack_world_core::{Library, NodeDraft};

use crate::support::routers;

#[tokio::test]
async fn should_not_advance_the_latest_library_when_installing_a_custom_file() {
    let world = generated_world(100).await;
    let latest = world
        .modules
        .libraries
        .generator
        .latest_full_library(LibraryKind::Ssh)
        .await
        .unwrap()
        .unwrap()
        .version;
    let folder = world
        .modules
        .core
        .files
        .lookup
        .folder(world.player(), "/lib", false)
        .await
        .unwrap()
        .unwrap();
    let mut file = NodeDraft::file(
        "libssh.so",
        ROOT_USER,
        PermissionSet::default_for(false),
        FileData::library(0),
    );

    file.library = Some(Box::new(Library {
        kind: LibraryKind::Ssh,
        version: "9.9.9".to_owned(),
    }));
    world
        .modules
        .core
        .files
        .changes
        .add_node(&folder, &file)
        .await
        .unwrap();

    assert_eq!(
        world
            .modules
            .libraries
            .generator
            .latest_full_library(LibraryKind::Ssh)
            .await
            .unwrap()
            .unwrap()
            .version,
        latest
    );
}

async fn generated_world(seed: u64) -> World {
    World::generate(&WorldOptions {
        seed,
        networks: 2,
        ..WorldOptions::default()
    })
    .await
    .expect("a generated world")
}

#[tokio::test]
async fn should_not_return_a_cached_library_for_a_reused_database_id() {
    let world = World::new(&greyhack_world::PlayerOptions::default())
        .await
        .unwrap();
    let first = greyhack_world_libraries::Library {
        kind: LibraryKind::Crypto,
        version: "200.0.0".to_owned(),
        zones: Vec::new(),
    };

    world.begin().await.unwrap();
    world
        .modules
        .libraries
        .updates
        .register_library(&first)
        .await
        .unwrap();
    let id = world
        .modules
        .core
        .files
        .latest_library(LibraryKind::Crypto)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        world
            .modules
            .libraries
            .generator
            .latest_full_library(LibraryKind::Crypto)
            .await
            .unwrap(),
        Some(first)
    );

    // Rolling back removes the head and rewinds AUTOINCREMENT. A core identity write can then reuse the cached ID
    // without any notification to the full-library repository.
    world.database().rollback().await.unwrap();
    world.database().begin().await.unwrap();
    let reused = world
        .modules
        .core
        .files
        .library_row(LibraryKind::Crypto, "201.0.0")
        .await
        .unwrap();

    assert_eq!(reused, id);
    assert_eq!(
        world
            .modules
            .libraries
            .generator
            .latest_full_library(LibraryKind::Crypto)
            .await
            .unwrap()
            .unwrap()
            .version,
        "201.0.0"
    );
}

#[tokio::test]
async fn should_prepare_the_initial_history_and_metaxploit_versions() {
    let world = generated_world(41).await;

    for kind in LibraryKind::ALL.iter().copied() {
        let versions = world
            .modules
            .libraries
            .versions
            .saved_library_versions(kind)
            .await
            .unwrap();
        let expected = if kind == LibraryKind::Metaxploit {
            91
        } else {
            6
        };

        assert_eq!(versions.len(), expected, "{kind:?}");
        assert_eq!(versions.first().map(String::as_str), Some(FIRST_VERSION));
    }

    let metaxploit = world
        .modules
        .libraries
        .versions
        .saved_library_versions(LibraryKind::Metaxploit)
        .await
        .unwrap();

    assert_eq!(metaxploit.last().map(String::as_str), Some("1.9.0"));

    world
        .modules
        .libraries
        .versions
        .prepare_initial_versions()
        .await
        .unwrap();

    assert_eq!(
        world
            .modules
            .libraries
            .versions
            .saved_library_versions(LibraryKind::Metaxploit)
            .await
            .unwrap()
            .len(),
        91
    );
}

#[tokio::test]
async fn should_choose_saved_versions_deterministically_and_share_them_in_a_network() {
    let world = generated_world(42).await;
    let other = generated_world(42).await;
    let world_routers = routers(&world).await;
    let other_routers = routers(&other).await;

    for (router, other_router) in world_routers.iter().zip(&other_routers) {
        let choices = world
            .modules
            .libraries
            .updates
            .network_library_versions(router.network)
            .await
            .unwrap();
        let other_choices = other
            .modules
            .libraries
            .updates
            .network_library_versions(other_router.network)
            .await
            .unwrap();

        assert_eq!(choices, other_choices);
        assert_eq!(choices.len(), LibraryKind::ALL.len());

        for computer in world
            .modules
            .core
            .computers
            .computers_in(router.network)
            .await
            .unwrap()
        {
            if computer.id == world.player() {
                continue;
            }

            for (kind, version) in &choices {
                let path = greyhack_behaviour::library_path(kind.file_name());
                let Some(library) = world
                    .modules
                    .libraries
                    .generator
                    .full_file_library(computer.id, &path, false)
                    .await
                    .unwrap()
                else {
                    continue;
                };

                assert_eq!(&library.version, version);
            }
        }
    }

    let player = world.player();

    for kind in LibraryKind::ALL.iter().copied() {
        let Some(library) = world
            .modules
            .libraries
            .generator
            .full_file_library(
                player,
                &greyhack_behaviour::library_path(kind.file_name()),
                false,
            )
            .await
            .unwrap()
        else {
            continue;
        };
        let saved = world
            .modules
            .libraries
            .versions
            .saved_library_versions(kind)
            .await
            .unwrap();

        assert!(saved.contains(&library.version));
    }
}

#[tokio::test]
async fn should_scan_v1_hidden_zones_with_a_fresh_metaxploit() {
    let world = generated_world(43).await;
    let library = world
        .modules
        .libraries
        .generator
        .full_file_library(
            world.player(),
            &greyhack_behaviour::library_path(LibraryKind::Metaxploit.file_name()),
            false,
        )
        .await
        .unwrap()
        .expect("the player's metaxploit");

    // `Library.Vulnerabilidad` defaults to 0.0.9, so the first version can read the first hidden zones.
    assert!(
        library
            .zones
            .iter()
            .filter(|zone| zone.hide)
            .flat_map(|zone| &zone.weaknesses)
            .any(|weakness| weakness.metaxploit_version == "0.0.9")
    );

    let scan = world
        .modules
        .libraries
        .metaxploit
        .scan_library(&library, FIRST_VERSION)
        .await
        .unwrap()
        .expect("the metaxploit scan");

    assert!(!scan.hidden_zones);
}

#[tokio::test]
async fn should_keep_the_saved_history_at_the_cap() {
    let world = generated_world(44).await;

    for _ in 0..100 {
        world
            .modules
            .libraries
            .versions
            .release_library_version(LibraryKind::Ssh)
            .await
            .unwrap();
    }

    assert_eq!(
        world
            .modules
            .libraries
            .versions
            .saved_library_versions(LibraryKind::Ssh)
            .await
            .unwrap()
            .len(),
        100
    );
}
