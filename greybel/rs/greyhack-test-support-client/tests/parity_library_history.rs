use greybel_test_support::game::Game;
use greyhack_behaviour::LibraryKind;

use crate::support::game::{join_fixture, setup_player, ssh_network_of_kind, test_client};
use crate::support::intrinsics::snapshot_world;
use crate::support::mirror;

/// Saved library contents and the router-node's choices are visible in the mirrored world.
#[tokio::test]
async fn should_mirror_library_history_and_network_choices() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;

    for kind in ["RentServer", "Supermercados", "Comisaria"] {
        let network = ssh_network_of_kind(&client, kind).await;
        let snapshot = join_fixture(&client, &network).await;
        let user = setup_player(&client).await;

        assert_eq!(
            snapshot.library_history.len(),
            LibraryKind::ALL.len(),
            "{kind}"
        );
        assert_eq!(
            snapshot.library_choices.len(),
            LibraryKind::ALL.len(),
            "{kind}"
        );
        assert!(
            snapshot
                .library_history
                .iter()
                .all(|history| !history.contents.is_empty()),
            "{kind}: every kind has a saved history"
        );

        for choice in &snapshot.library_choices {
            let history = snapshot
                .library_history
                .iter()
                .find(|history| history.kind == choice.kind)
                .expect("the choice has a history");
            let version_exists = history.contents.iter().any(|content| {
                mirror::library(content).is_some_and(|library| library.version == choice.version)
            });

            assert!(version_exists, "{kind}: {}@{}", choice.kind, choice.version);
        }

        let layout = mirror::layout(&snapshot);
        let choices = layout.library_choices.clone();
        let world = snapshot_world(&user, &snapshot);
        let router = world
            .modules
            .core
            .networks
            .router_at(&snapshot.public_ip)
            .await
            .unwrap()
            .expect("the mirrored router");
        let choices_in_world = world
            .modules
            .libraries
            .updates
            .network_library_versions(router.network)
            .await
            .unwrap();

        assert_eq!(choices_in_world, choices, "{kind}: router-node draw");
        assert_eq!(choices_in_world.len(), LibraryKind::ALL.len());

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

            for (kind, version) in &choices_in_world {
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

                assert_eq!(
                    &library.version, version,
                    "{kind:?}: shared network version"
                );
            }
        }
    }
}
