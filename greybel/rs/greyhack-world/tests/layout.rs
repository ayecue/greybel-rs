use greyhack_behaviour::{DeviceKind, FileData, PermissionSet, ROOT_USER, parse_date};
use greyhack_world::{PlayerOptions, World};
use greyhack_world_core::{NodeDraft, WorldError};
use greyhack_world_creation::DeviceLayout;

use crate::support::layout::layout;

/// A world whose player's network is laid out has exactly its nodes: the player on its computer, the devices and
/// computers in the tree as laid out, the forwards of the routers' tables.
#[tokio::test]
async fn should_build_the_players_network_as_laid_out() {
    let world = World::new(&PlayerOptions {
        network: Some(layout()),
        ..PlayerOptions::default()
    })
    .await
    .unwrap();
    let player = world
        .modules
        .core
        .networks
        .topology
        .computer(world.handle.player())
        .await
        .unwrap()
        .unwrap();
    let lan = world
        .modules
        .core
        .networks
        .lan
        .lan(player.network)
        .await
        .unwrap()
        .unwrap();
    let order: Vec<(DeviceKind, String)> = lan
        .search_order()
        .into_iter()
        .map(|id| (lan.node(id).kind, lan.node(id).local_ip.clone()))
        .collect();
    let main = world
        .modules
        .core
        .networks
        .topology
        .main_router(player.network)
        .await
        .unwrap()
        .unwrap();
    let forwards: Vec<(u16, String, u16)> = world
        .modules
        .core
        .networks
        .router_config
        .port_forwards(main.id)
        .await
        .unwrap()
        .into_iter()
        .map(|forward| (forward.number, forward.lan_ip, forward.port.number))
        .collect();

    assert_eq!(
        (player.local_ip.as_str(), main.public_ip.as_str()),
        ("192.168.1.6", "45.45.45.45")
    );
    assert_eq!(
        order,
        [
            (DeviceKind::Router, "192.168.1.1".to_owned()),
            (DeviceKind::Computer, "192.168.1.6".to_owned()),
            (DeviceKind::Router, "172.16.3.1".to_owned()),
            (DeviceKind::Computer, "172.16.3.7".to_owned()),
            (DeviceKind::Hub, String::new()),
            (DeviceKind::Computer, "172.16.3.9".to_owned()),
        ]
    );
    assert_eq!(forwards, [(2222, "172.16.3.7".to_owned(), 22)]);
}

/// Mirrored computers' explicit library files win over router-node choices. Existing computers are copied state, not
/// generated or restored computers to which library updates should be applied.
#[tokio::test]
async fn should_keep_mirrored_computers_exact_library_contents() {
    use greyhack_behaviour::{LibraryKind, Rng};
    use greyhack_world_libraries::{ComputerDraft, Library, LibraryGeneration};

    let mut network = layout();
    let expected_player = Library::generate(&mut Rng::new(918_273), LibraryKind::Crypto, "1.0.7");
    let expected_npc = Library::generate(&mut Rng::new(456_789), LibraryKind::Crypto, "1.0.8");
    let add_crypto = |draft: &mut ComputerDraft, library: Library| {
        let root = draft.filesystem.root();
        let lib = draft.filesystem.add(
            root,
            NodeDraft::folder("lib", ROOT_USER, PermissionSet::default_for(true)),
        );
        let mut crypto = NodeDraft::file(
            "crypto.so",
            ROOT_USER,
            PermissionSet::default_for(false),
            FileData::library(0),
        );

        crypto.library = Some(Box::new(library));
        draft.filesystem.add(lib, crypto);
    };
    let DeviceLayout::Computer(player) = &mut network.nodes[1].device else {
        panic!("the second node is the player")
    };
    add_crypto(player, expected_player.clone());
    let DeviceLayout::Computer(npc) = &mut network.nodes[3].device else {
        panic!("the fourth node is an NPC")
    };
    add_crypto(npc, expected_npc.clone());
    network.library_choices = vec![(LibraryKind::Crypto, "1.0.0".to_owned())];

    let world = World::new(&PlayerOptions {
        network: Some(network),
        ..PlayerOptions::default()
    })
    .await
    .unwrap();

    assert_eq!(
        world
            .modules
            .libraries
            .generator
            .full_file_library(world.player(), "/lib/crypto.so", false)
            .await
            .unwrap(),
        Some(expected_player)
    );
    let network = world
        .modules
        .core
        .networks
        .topology
        .computer(world.player())
        .await
        .unwrap()
        .unwrap()
        .network;
    let npc = world
        .modules
        .core
        .computers
        .computers_in(network)
        .await
        .unwrap()
        .into_iter()
        .find(|computer| computer.local_ip == "172.16.3.7")
        .unwrap();

    assert_eq!(
        world
            .modules
            .libraries
            .generator
            .full_file_library(npc.id, "/lib/crypto.so", false)
            .await
            .unwrap(),
        Some(expected_npc)
    );
}

#[tokio::test]
async fn should_refuse_a_layout_with_a_computer_under_no_device() {
    let mut network = layout();

    network.nodes[1].parent = None;

    let error = World::new(&PlayerOptions {
        network: Some(network),
        ..PlayerOptions::default()
    })
    .await
    .err();

    assert!(matches!(error, Some(WorldError::Layout(_))), "{error:?}");
}

#[tokio::test]
async fn should_refuse_a_layout_without_the_players_computer() {
    let mut network = layout();

    if let DeviceLayout::Computer(draft) = &mut network.nodes[1].device {
        draft.is_player = false;
    }

    let error = World::new(&PlayerOptions {
        network: Some(network),
        ..PlayerOptions::default()
    })
    .await
    .err();

    assert!(matches!(error, Some(WorldError::Layout(_))), "{error:?}");
}

/// A laid-out network's administrator is saved with the world.
#[tokio::test]
async fn should_save_a_laid_out_networks_administrator() {
    let mut network = layout();

    network.admin = Some(greyhack_behaviour::NetworkAdmin {
        intelligence: 10,
        pc_experience: 10,
        work_performance: greyhack_behaviour::WorkPerformance::Excellent,
        dead_or_fired: false,
    });

    let mut world = World::new(&PlayerOptions {
        network: Some(network),
        ..PlayerOptions::default()
    })
    .await
    .unwrap();
    let player = world
        .modules
        .core
        .networks
        .topology
        .computer(world.handle.player())
        .await
        .unwrap()
        .unwrap();
    let database = greybel_test_support::world::save_copy(&mut world).await;
    let restored = World::load(&database).await.unwrap();

    assert!(
        restored
            .modules
            .npcs
            .jobs
            .has_super_admin(player.network)
            .await
            .unwrap()
    );
}

/// The files of a laid-out computer keep their dates when its file system has absolute dates, and are shifted by the
/// computer's stable template offset otherwise (`XmlGlobal.CloneTemplate`).
#[tokio::test]
async fn should_keep_the_dates_of_a_laid_out_file_system_with_absolute_dates() {
    // Arrange
    let date = parse_date("05/Mar/2026 - 14:07");
    let mut dated = Vec::new();

    for absolute in [false, true] {
        let mut network = layout();
        let DeviceLayout::Computer(server) = &mut network.nodes[3].device else {
            panic!("a computer");
        };
        let mut file = NodeDraft::file(
            "dated",
            ROOT_USER,
            PermissionSet::default_for(false),
            FileData::text("date"),
        );

        file.timestamp = date;

        let root = server.filesystem.root();

        server.filesystem.add(root, file);

        if absolute {
            server.filesystem = std::mem::take(&mut server.filesystem).with_absolute_dates();
        }

        // Act
        let world = World::new(&PlayerOptions {
            network: Some(network),
            ..PlayerOptions::default()
        })
        .await
        .unwrap();
        let computer = world
            .modules
            .core
            .networks
            .routers
            .computer_at("45.45.45.45", "172.16.3.7")
            .await
            .unwrap()
            .unwrap();
        let node = world
            .modules
            .core
            .files
            .lookup
            .file(computer, "/dated", false)
            .await
            .unwrap()
            .unwrap();

        dated.push(node.timestamp);
    }

    // Assert
    assert!(dated[0] < date, "the template shift ages the file");
    assert_eq!(dated[1], date);
}
