use greyhack_behaviour::{DeviceKind, FileKind, PermissionSet};
use greyhack_world::{NetworkOptions, PlayerOptions, World};
use greyhack_world_assets::WorldItemTable;

use crate::support::computers;
use crate::support::random_content::{generated, random_files};

#[tokio::test]
async fn should_add_random_files_to_the_homes_of_generated_computers() {
    let (world, computers) = generated(60).await;
    let items = WorldItemTable::get();
    let mut kinds = std::collections::BTreeSet::new();
    let mut with_files = 0;

    for computer in computers {
        let files = random_files(&world, computer).await;

        with_files += usize::from(!files.is_empty());
        assert!(files.len() <= 4, "{files:?}");

        for (path, kind, owner, group, permissions) in files {
            let (home, name) = path.rsplit_once('/').unwrap();

            assert_eq!(home, format!("/home/{owner}"));
            assert_eq!(group, owner);
            assert_eq!(permissions, PermissionSet::default_for(false).as_str());

            match kind {
                FileKind::ChatLog => assert!(name.starts_with("chat") && name.ends_with(".log")),
                FileKind::Pdf => assert!(
                    items
                        .texts
                        .iter()
                        .chain(&items.neurobox_texts)
                        .any(|item| item.name == name)
                ),
                _ => assert!(items.images.iter().any(|item| item.name == name)),
            }

            kinds.insert(kind.code());
        }
    }

    // Half of the computers draw the file branch, three in ten an image.
    assert!(with_files > 10, "{with_files}");
    assert!(kinds.len() >= 2, "{kinds:?}");
}

#[tokio::test]
async fn should_add_the_same_random_files_to_the_same_world() {
    let (first, computers) = generated(12).await;
    let (second, _) = generated(12).await;

    for computer in computers {
        assert_eq!(
            random_files(&first, computer).await,
            random_files(&second, computer).await
        );
    }
}

#[tokio::test]
async fn should_add_random_content_to_planned_networks_but_not_to_devices() {
    let world = World::new(&PlayerOptions::default()).await.unwrap();

    for _ in 0..12 {
        world
            .modules
            .creation
            .network_generation
            .add_network(&NetworkOptions::default())
            .await
            .unwrap();
    }

    let mut devices = 0;
    let mut with_files = 0;
    let mut conversations = 0;

    for computer in computers(&world).await {
        let files = random_files(&world, computer.id).await;

        match computer.kind {
            DeviceKind::Computer => {
                with_files += usize::from(!files.is_empty());

                for npc in world.modules.npcs.npcs.npcs_on(computer.id).await.unwrap() {
                    let address = world
                        .modules
                        .npcs
                        .mail
                        .mail_address(npc.person)
                        .await
                        .unwrap()
                        .unwrap();
                    let inbox = world
                        .modules
                        .npcs
                        .mail
                        .mail_account(&address)
                        .await
                        .unwrap()
                        .unwrap()
                        .inbox;

                    for mail in inbox {
                        // A conversation is one mail of several messages.
                        assert_eq!(mail.subject, "RE: stuff");
                        assert!(!mail.replies.is_empty());
                        assert!(mail.from.contains('@'));
                        conversations += 1;
                    }
                }
            }
            DeviceKind::Camera
            | DeviceKind::Fridge
            | DeviceKind::Microwave
            | DeviceKind::Smartphone => {
                devices += 1;
                assert!(files.is_empty());
            }
            _ => {}
        }
    }

    assert!(devices > 0);
    assert!(with_files > 0);
    assert!(conversations > 0);
}

/// The administrator restoring the network regenerates its computers with their random files
/// (`RedGlobal.AdminRevertComputer` runs `Router.GeneraLanPc`).
#[tokio::test]
async fn should_regenerate_the_random_files_when_an_admin_restores_a_computer() {
    let world = World::new(&PlayerOptions::default()).await.unwrap();
    let mut target = None;

    for _ in 0..40 {
        let router = world
            .modules
            .creation
            .network_generation
            .add_network(&NetworkOptions {
                computers: Some(3),
                ..NetworkOptions::default()
            })
            .await
            .unwrap();

        for candidate in computers(&world).await {
            if candidate.router == Some(router)
                && !world
                    .modules
                    .npcs
                    .npcs
                    .npcs_on(candidate.id)
                    .await
                    .unwrap()
                    .is_empty()
                && !random_files(&world, candidate.id).await.is_empty()
            {
                target = Some(candidate.id);
            }
        }

        if target.is_some() {
            break;
        }
    }

    let computer = target.expect("a computer with NPCs and random files");
    let before = random_files(&world, computer).await;

    // A missing /lib is one of the game's ShouldRestore triggers (`RedGlobal.ShouldRestore`).
    let lib = world
        .modules
        .core
        .files
        .lookup
        .folder(computer, "/lib", false)
        .await
        .unwrap()
        .unwrap();

    world
        .modules
        .core
        .files
        .changes
        .remove_node(&lib)
        .await
        .unwrap();

    for (path, ..) in &before {
        let file = world
            .modules
            .core
            .files
            .lookup
            .file(computer, path, false)
            .await
            .unwrap()
            .unwrap();

        world
            .modules
            .core
            .files
            .changes
            .remove_node(&file)
            .await
            .unwrap();
    }

    assert!(random_files(&world, computer).await.is_empty());

    for _ in 0..400 {
        let now = world.handle.time_seconds();
        let Some(next) = world.modules.routines.next_due_at().await.unwrap() else {
            break;
        };

        world.tick(next.saturating_sub(now)).await.unwrap();

        if world
            .modules
            .core
            .files
            .lookup
            .folder(computer, "/lib", false)
            .await
            .unwrap()
            .is_some()
        {
            break;
        }
    }

    assert_eq!(random_files(&world, computer).await, before);
}
