use greyhack_behaviour::{FileKind, LibraryKind};
use greyhack_world::{ComputerOptions, LibraryReportFailure, NetworkOptions, World};

use crate::support::library_updates::{
    FTP_PATH, SSH, SSH_PATH, library_at, network_of, ssh_port_version, version_of, world,
};
use crate::support::{computers, router, save_copy};

/// A report gives the library files of the network that are older than the newest library of the kind that
/// library, and no other file (`FileSystem.ApplyReportedLibUpdates`).
#[tokio::test]
async fn should_update_the_older_library_files_of_the_network_to_the_reported_library() {
    // Arrange
    let mut world = world().await;
    let (public_ip, found) = network_of(&mut world, &["1.0.0", "1.5.0", "2.0.0"]).await;
    let (other_ip, other) = network_of(&mut world, &["1.0.0"]).await;
    let ftp_before = version_of(&world, found[0], FTP_PATH).await;

    assert_ne!(public_ip, other_ip);

    // Act
    let reported = world.report_library_update(&public_ip, SSH).await.unwrap();

    // Assert
    assert_eq!(reported, Some(Ok(())));
    assert_eq!(version_of(&world, found[0], SSH_PATH).await, "2.0.0");
    assert_eq!(version_of(&world, found[1], SSH_PATH).await, "2.0.0");
    assert_eq!(version_of(&world, found[2], SSH_PATH).await, "2.0.0");
    assert_eq!(version_of(&world, found[0], FTP_PATH).await, ftp_before);
    assert_eq!(version_of(&world, other[0], SSH_PATH).await, "1.0.0");
}

/// The file's library is the reported one, with its weaknesses, not only its version number.
#[tokio::test]
async fn should_give_the_files_the_reported_library_with_its_weaknesses() {
    // Arrange
    let mut world = world().await;
    let (public_ip, found) = network_of(&mut world, &["1.0.0", "2.0.0"]).await;
    let newest = world
        .latest_library(LibraryKind::Ssh)
        .await
        .unwrap()
        .unwrap();

    // Act
    world.report_library_update(&public_ip, SSH).await.unwrap();

    // Assert
    let updated = world
        .file_library(found[0], SSH_PATH, false)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(updated, newest);
    assert!(!updated.zones.is_empty());
}

/// There is no report without a network, or for a name that is not one of the world's libraries; nothing changes.
#[tokio::test]
async fn should_report_nothing_for_a_missing_network_or_an_unknown_library() {
    // Arrange
    let mut world = world().await;
    let (public_ip, found) = network_of(&mut world, &["1.0.0", "2.0.0"]).await;

    // Act
    let missing = world.report_library_update("1.2.3.4", SSH).await.unwrap();
    let unknown = world
        .report_library_update(&public_ip, "libnothing.so")
        .await
        .unwrap();
    let without_suffix = world
        .report_library_update(&public_ip, "libssh")
        .await
        .unwrap();

    // Assert
    assert_eq!(missing, None);
    assert_eq!(unknown, Some(Err(LibraryReportFailure::UnknownLibrary)));
    assert_eq!(
        without_suffix,
        Some(Err(LibraryReportFailure::UnknownLibrary))
    );
    assert_eq!(version_of(&world, found[0], SSH_PATH).await, "1.0.0");
}

/// A report does not change the player's computer, which the game never restores through the update
/// (`Database.RestoreComputer`).
#[tokio::test]
async fn should_not_update_the_player_computer() {
    // Arrange
    let mut world = world().await;
    let player = world.player();
    let player_ip = {
        let own = computers(&world)
            .await
            .into_iter()
            .find(|computer| computer.id == player)
            .expect("the player computer");

        router(&world, own.router.expect("the player's router"))
            .await
            .public_ip
    };
    let before = version_of(&world, player, SSH_PATH).await;

    world
        .register_library(&library_at(LibraryKind::Ssh, "7.0.0"))
        .await
        .unwrap();

    // Act
    let reported = world.report_library_update(&player_ip, SSH).await.unwrap();

    // Assert
    assert_eq!(reported, Some(Ok(())));
    assert_eq!(version_of(&world, player, SSH_PATH).await, before);
}

/// A computer made for the network after the report starts with the reported library, its port with that version
/// (`ServerMap.HelperLibVersions.SetLastLibVersion`); one of another network does not.
#[tokio::test]
async fn should_start_the_computers_made_after_a_report_with_the_reported_library() {
    // Arrange
    let mut world = world().await;
    let (public_ip, _) = network_of(&mut world, &["1.0.0", "2.0.0"]).await;
    let router_id = world.router_at(&public_ip).await.unwrap().unwrap().id;
    let (other_ip, _) = network_of(&mut world, &[]).await;
    let other_router = world.router_at(&other_ip).await.unwrap().unwrap().id;
    let options = ComputerOptions {
        services: Some(vec!["ssh".to_owned()]),
        ..ComputerOptions::default()
    };

    world.report_library_update(&public_ip, SSH).await.unwrap();

    // Act
    let made = world
        .add_computer_to(router_id, &options)
        .await
        .unwrap()
        .unwrap();
    let elsewhere = world
        .add_computer_to(other_router, &options)
        .await
        .unwrap()
        .unwrap();

    // Assert
    assert_eq!(version_of(&world, made, SSH_PATH).await, "2.0.0");
    assert_eq!(ssh_port_version(&world, made).await, "2.0.0");
    assert_ne!(version_of(&world, elsewhere, SSH_PATH).await, "2.0.0");
}

/// What a network reported is kept when the world is saved and loaded.
#[tokio::test]
async fn should_keep_the_reported_libraries_when_the_world_is_loaded() {
    // Arrange
    let mut world = world().await;
    let (public_ip, _) = network_of(&mut world, &["1.0.0", "2.0.0"]).await;
    let router_id = world.router_at(&public_ip).await.unwrap().unwrap().id;

    world.report_library_update(&public_ip, SSH).await.unwrap();

    let database = save_copy(&mut world).await;
    let mut loaded = World::load(&database).await.unwrap();

    // Act
    let made = loaded
        .add_computer_to(
            router_id,
            &ComputerOptions {
                services: Some(vec!["ssh".to_owned()]),
                ..ComputerOptions::default()
            },
        )
        .await
        .unwrap()
        .unwrap();

    // Assert
    assert_eq!(version_of(&loaded, made, SSH_PATH).await, "2.0.0");
}

/// The administrator regenerating a computer gives it the library its network reported, not the one it was generated
/// with (`RedGlobal.AdminRevertComputer`, `LanPcInfo.UpdateLibVersion`).
#[tokio::test]
async fn should_regenerate_a_computer_with_the_reported_library_when_an_admin_restores_it() {
    // Arrange
    let mut world = world().await;
    let mut target = None;

    for _ in 0..40 {
        let router_id = world
            .add_network(&NetworkOptions {
                computers: Some(3),
                ..NetworkOptions::default()
            })
            .await
            .unwrap();
        let public_ip = router(&world, router_id).await.public_ip;

        for candidate in computers(&world).await {
            if candidate.router == Some(router_id)
                && !world.npcs_on(candidate.id).await.unwrap().is_empty()
                && world
                    .folder(candidate.id, "/lib", false)
                    .await
                    .unwrap()
                    .is_some()
            {
                target = Some((public_ip.clone(), candidate.id));
            }
        }

        if target.is_some() {
            break;
        }
    }

    let (public_ip, computer) = target.expect("a computer with NPCs and libraries");
    let folder = world
        .folder(computer, "/lib", false)
        .await
        .unwrap()
        .unwrap();
    let (name, kind) = world
        .files(&folder)
        .await
        .unwrap()
        .into_iter()
        .find_map(|file| {
            let is_library = file
                .data()
                .is_some_and(|data| data.kind == FileKind::Library);

            LibraryKind::of_file(file.name())
                .filter(|_| is_library)
                .map(|kind| (file.name().to_owned(), kind))
        })
        .expect("a library file");

    world
        .register_library(&library_at(kind, "9.0.0"))
        .await
        .unwrap();
    world
        .report_library_update(&public_ip, &name)
        .await
        .unwrap();

    let path = format!("/lib/{name}");

    assert_eq!(version_of(&world, computer, &path).await, "9.0.0");

    // A missing /lib is one of the game's ShouldRestore triggers (`RedGlobal.ShouldRestore`).
    world.remove_node(&folder).await.unwrap();

    // Act
    for _ in 0..400 {
        let now = world.time_seconds();
        let Some(next) = world.next_due_at().await.unwrap() else {
            break;
        };

        world.tick(next.saturating_sub(now)).await.unwrap();

        if world
            .folder(computer, "/lib", false)
            .await
            .unwrap()
            .is_some()
        {
            break;
        }
    }

    // Assert
    assert_eq!(version_of(&world, computer, &path).await, "9.0.0");
}
