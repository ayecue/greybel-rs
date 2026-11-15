use greyhack_behaviour::{
    APT_CACHE_PATH, APT_PACKAGE_SIZE, APT_SOURCES_PATH, DEFAULT_REPOSITORY,
    DEFAULT_REPOSITORY_PORT, FileData, FileKind, GUEST_USER, LibraryKind, NetworkKind, ROOT_USER,
    ServiceId, format_apt_sources,
};
use greyhack_world::{PlayerOptions, World};
use greyhack_world_core::{ConnectionFailure, Port};
use greyhack_world_creation::DeviceLayout;
use greyhack_world_libraries::Library;
use greyhack_world_services::{AptFailure, AptUpdateLine, PackageEntry, RepositoryConfig};

use crate::support::apt::{file, online, put, shop_file_names, world_with_shop_source};
use crate::support::generation::add_typed_network;
use crate::support::layout::layout;

#[tokio::test]
async fn should_edit_sources_and_return_update_lines() {
    let world = World::new(&PlayerOptions::default()).await.unwrap();
    let computer = world.handle.player();
    let address = "45.45.45.45";

    assert_eq!(
        world
            .modules
            .services
            .apt
            .add_apt_repository(computer, DEFAULT_REPOSITORY, DEFAULT_REPOSITORY_PORT)
            .await
            .unwrap(),
        Err(AptFailure::InvalidAddress(DEFAULT_REPOSITORY.to_owned()))
    );
    assert_eq!(
        world
            .modules
            .services
            .apt
            .add_apt_repository(computer, address, DEFAULT_REPOSITORY_PORT)
            .await
            .unwrap(),
        Ok(())
    );
    assert_eq!(
        world
            .modules
            .services
            .apt
            .add_apt_repository(computer, address, DEFAULT_REPOSITORY_PORT)
            .await
            .unwrap(),
        Err(AptFailure::ListedRepository(address.to_owned()))
    );
    assert_eq!(
        world
            .modules
            .services
            .apt
            .update_apt_packages(computer, &[])
            .await
            .unwrap(),
        vec![
            AptUpdateLine::Update(address.to_owned()),
            AptUpdateLine::Connection(ConnectionFailure::MissingPort(DEFAULT_REPOSITORY_PORT)),
        ]
    );

    assert_eq!(
        world
            .modules
            .services
            .apt
            .remove_apt_repository(computer, address)
            .await
            .unwrap(),
        Ok(())
    );
    assert_eq!(
        world
            .modules
            .core
            .files
            .lookup
            .content(computer, APT_SOURCES_PATH, true)
            .await
            .unwrap(),
        Some(format_apt_sources(&[], true))
    );
    assert!(
        world
            .modules
            .services
            .apt
            .update_apt_packages(computer, &[])
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        world
            .modules
            .services
            .apt
            .remove_apt_repository(computer, address)
            .await
            .unwrap(),
        Err(AptFailure::UnlistedRepository(address.to_owned()))
    );
}

#[tokio::test]
async fn should_search_and_show_repository_packages() {
    let world = World::new(&PlayerOptions::default()).await.unwrap();
    let computer = world.handle.player();

    world
        .modules
        .services
        .apt
        .update_apt_packages(computer, &[])
        .await
        .unwrap();

    assert_eq!(
        world
            .modules
            .services
            .apt
            .search_apt_packages(computer, "blockchain", &[])
            .await
            .unwrap(),
        Ok(vec![PackageEntry {
            name: "blockchain.so".to_owned(),
            description: None,
        }])
    );
    assert_eq!(
        world
            .modules
            .services
            .apt
            .search_apt_packages(computer, "missing", &[])
            .await
            .unwrap(),
        Err(AptFailure::NoMatch("missing".to_owned()))
    );

    let official = world
        .modules
        .services
        .apt
        .show_apt_repository(computer, "official_server", &[])
        .await
        .unwrap()
        .unwrap();

    for kind in LibraryKind::ALL {
        if !["metaxploit.so", "crypto.so", "librshell.so"].contains(&kind.file_name().as_str()) {
            assert!(
                official
                    .iter()
                    .any(|package| package.name == kind.file_name())
            );
        }
    }

    assert_eq!(
        world
            .modules
            .services
            .apt
            .show_apt_repository(computer, "missing.test", &[])
            .await
            .unwrap(),
        Err(AptFailure::UnknownRepository("missing.test".to_owned()))
    );
}

#[tokio::test]
async fn should_install_a_library_with_its_owner_and_default_folder() {
    let world = World::new(&PlayerOptions::default()).await.unwrap();
    let computer = world.handle.player();

    world
        .modules
        .services
        .apt
        .update_apt_packages(computer, &[])
        .await
        .unwrap();

    assert_eq!(
        world
            .modules
            .services
            .apt
            .install_apt_package(computer, ROOT_USER, "nmap", "", &[])
            .await
            .unwrap(),
        Err(AptFailure::UnknownPackage("nmap".to_owned()))
    );
    assert_eq!(
        world
            .modules
            .services
            .apt
            .install_apt_package(computer, ROOT_USER, "blockchain.so", "", &[])
            .await
            .unwrap(),
        Ok(())
    );

    let node = world
        .modules
        .core
        .files
        .lookup
        .file(computer, "/lib/blockchain.so", false)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(node.owner, ROOT_USER);
    assert_eq!(node.data().unwrap().size, APT_PACKAGE_SIZE);
    assert!(
        world
            .modules
            .libraries
            .generator
            .full_library_of(&node)
            .await
            .unwrap()
            .is_some()
    );

    assert_eq!(
        world
            .modules
            .services
            .apt
            .install_apt_package(computer, ROOT_USER, "missing", "", &[])
            .await
            .unwrap(),
        Err(AptFailure::UnknownPackage("missing".to_owned()))
    );
    assert_eq!(
        world
            .modules
            .services
            .apt
            .install_apt_package(computer, ROOT_USER, "blockchain.so", "/missing", &[])
            .await
            .unwrap(),
        Err(AptFailure::MissingFolder("/missing".to_owned()))
    );
    assert_eq!(
        world
            .modules
            .services
            .apt
            .install_apt_package(computer, GUEST_USER, "blockchain.so", "/root", &[])
            .await
            .unwrap(),
        Err(AptFailure::NoPermission)
    );
}

#[tokio::test]
async fn should_detect_a_newer_library_in_the_sources() {
    let world = World::new(&PlayerOptions::default()).await.unwrap();
    let computer = world.handle.player();
    let path = "/lib/blockchain.so";
    let package = world
        .modules
        .services
        .apt
        .package(DEFAULT_REPOSITORY, "blockchain.so")
        .await
        .unwrap()
        .unwrap();
    let node = world
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
        .set_node_library(node.id, package.library)
        .await
        .unwrap();
    world
        .modules
        .services
        .apt
        .update_apt_packages(computer, &[])
        .await
        .unwrap();

    assert_eq!(
        world
            .modules
            .services
            .apt
            .check_apt_upgrade(computer, path, &[])
            .await
            .unwrap(),
        Ok(false)
    );
    assert!(
        world
            .modules
            .core
            .computers
            .hardware
            .set_library_version(computer, path, "0.0.1")
            .await
            .unwrap()
    );
    assert_eq!(
        world
            .modules
            .services
            .apt
            .check_apt_upgrade(computer, path, &[])
            .await
            .unwrap(),
        Ok(true)
    );
}

/// `AptClientIntrinsics.check_upgrade` needs network access before it looks at the cache.
#[tokio::test]
async fn should_refuse_to_check_an_upgrade_without_network_access() {
    let world = World::new(&PlayerOptions::default()).await.unwrap();
    let computer = world.handle.player();
    let mut card = world
        .modules
        .core
        .networks
        .net_cards
        .net_cards(computer)
        .await
        .unwrap()[0]
        .clone();

    card.active = false;
    world
        .modules
        .core
        .networks
        .net_cards
        .save_net_card(computer, 0, &card)
        .await
        .unwrap();

    assert_eq!(
        world
            .modules
            .services
            .apt
            .check_apt_upgrade(computer, "/lib/blockchain.so", &[])
            .await
            .unwrap(),
        Err(AptFailure::NoInternet),
        "without a cache the network is refused first"
    );
}

/// The checks of `AptClientIntrinsics.check_upgrade` in the order the game makes them: the cache, the name, the
/// official server's file, the sources.
#[tokio::test]
async fn should_refuse_to_check_an_upgrade_in_the_order_the_game_checks() {
    let world = World::new(&PlayerOptions::default()).await.unwrap();
    let computer = world.handle.player();

    assert_eq!(
        world
            .modules
            .services
            .apt
            .check_apt_upgrade(computer, "/lib/blockchain.so", &[])
            .await
            .unwrap(),
        Err(AptFailure::NoCache)
    );

    world
        .modules
        .services
        .apt
        .update_apt_packages(computer, &[])
        .await
        .unwrap();

    assert_eq!(
        world
            .modules
            .services
            .apt
            .check_apt_upgrade(computer, "", &[])
            .await
            .unwrap(),
        Err(AptFailure::EmptyPackageName)
    );
    assert_eq!(
        world
            .modules
            .services
            .apt
            .check_apt_upgrade(computer, "/lib/ghost/blockchain.so", &[])
            .await
            .unwrap(),
        Err(AptFailure::MissingLocalFile("blockchain.so".to_owned())),
        "the official server has the library, the computer does not"
    );
    assert_eq!(
        world
            .modules
            .services
            .apt
            .check_apt_upgrade(computer, "/lib/missing.so", &[])
            .await
            .unwrap(),
        Err(AptFailure::UnknownPackage("missing.so".to_owned()))
    );
}

#[tokio::test]
async fn should_refuse_to_check_an_upgrade_with_a_cache_that_is_not_one() {
    let world = World::new(&PlayerOptions::default()).await.unwrap();
    let computer = world.handle.player();

    world
        .modules
        .services
        .apt
        .update_apt_packages(computer, &[])
        .await
        .unwrap();

    let mut cache = world
        .modules
        .core
        .files
        .lookup
        .file(computer, APT_CACHE_PATH, true)
        .await
        .unwrap()
        .unwrap();

    cache.data_mut().unwrap().set_content("[1]");
    world
        .modules
        .core
        .files
        .changes
        .save_node(&cache)
        .await
        .unwrap();

    assert_eq!(
        world
            .modules
            .services
            .apt
            .check_apt_upgrade(computer, "/lib/blockchain.so", &[])
            .await
            .unwrap(),
        Err(AptFailure::MalformedCache)
    );
}

/// A server of the cache is asked in the order of the cache, and its failures are the answer.
#[tokio::test]
async fn should_refuse_to_check_an_upgrade_when_the_cached_server_fails() {
    let (world, player, shop) = world_with_shop_source().await;
    let path = "/Public/htdocs/downloads/scanlib";
    let local = "/root/scanlib";

    assert_eq!(
        world
            .modules
            .services
            .apt
            .check_apt_upgrade(player, local, &[])
            .await
            .unwrap(),
        Err(AptFailure::MissingLocalFile("scanlib".to_owned())),
        "the shop has the package and the computer does not"
    );

    let package = world
        .modules
        .core
        .files
        .lookup
        .file(shop, path, false)
        .await
        .unwrap()
        .unwrap();

    world
        .modules
        .core
        .files
        .changes
        .remove_node(&package)
        .await
        .unwrap();

    assert_eq!(
        world
            .modules
            .services
            .apt
            .check_apt_upgrade(player, local, &[])
            .await
            .unwrap(),
        Err(AptFailure::MissingPackageFile("scanlib".to_owned())),
        "the server lists a package it has no file of"
    );
}

/// `Router.PostGenComputer`: a HackShop's repository serves `/Public/htdocs/downloads` after
/// `PlayerUtils.UpdateHackshopLibs` added the libraries it sells, each described by its English detail.
#[tokio::test]
async fn should_generate_the_repository_catalog_of_a_hackshop_from_its_downloads() {
    let mut world = World::new(&PlayerOptions::default()).await.unwrap();
    let (_, shop) = add_typed_network(&mut world, NetworkKind::HackShop, "shop.test").await;
    let names = shop_file_names(&world, shop).await;
    let config: RepositoryConfig = serde_json::from_str(
        &world
            .modules
            .core
            .files
            .lookup
            .content(shop, ServiceId::Repository.config_path(), false)
            .await
            .unwrap()
            .unwrap(),
    )
    .unwrap();

    assert_eq!(
        names,
        [
            "AdminMonitor.exe",
            "decipher",
            "scanlib",
            "scanrouter",
            "sniffer",
            "rshell-server",
            "rshell-interface",
            "metaxploit.so",
            "crypto.so",
            "librshell.so",
        ]
    );
    assert_eq!(config.path_repo, "/Public/htdocs/downloads");
    assert_eq!(
        config.repo_sources.keys().collect::<Vec<_>>(),
        names.iter().collect::<Vec<_>>()
    );
    assert_eq!(
        config.repo_sources["metaxploit.so"],
        "Library analysis tool"
    );
    assert_eq!(
        config.repo_sources["crypto.so"],
        "Crypto suite <color=#6DD3A0FF>library</color>"
    );

    for kind in [
        LibraryKind::Metaxploit,
        LibraryKind::Crypto,
        LibraryKind::Rshell,
    ] {
        let file = world
            .modules
            .core
            .files
            .lookup
            .file(
                shop,
                &format!("/Public/htdocs/downloads/{}", kind.file_name()),
                false,
            )
            .await
            .unwrap()
            .unwrap();

        assert_eq!(file.data().unwrap().kind, FileKind::Library);
        assert_eq!(
            world
                .modules
                .libraries
                .generator
                .full_library_of(&file)
                .await
                .unwrap(),
            world
                .modules
                .libraries
                .generator
                .latest_full_library(kind)
                .await
                .unwrap()
        );
    }
}

/// `AptClientIntrinsics.search` and `install`: a custom source adds to the official server instead of replacing it,
/// and its packages are the files of the repository computer's `path_repo`.
#[tokio::test]
async fn should_list_and_install_a_hackshop_repository_next_to_the_official_server() {
    let (world, player, _) = world_with_shop_source().await;

    assert_eq!(
        world.modules.services.apt.search_apt_packages(player, "scanlib", &[]).await.unwrap(),
        Ok(vec![PackageEntry {
            name: "scanlib".to_owned(),
            description: Some(
                "Analyze the provided <color=#6DD3A0FF>library</color> to <color=#FF8F00FF>search</color> for \
                 <color=#6DD3A0FF>vulnerabilities</color>."
                    .to_owned()
            ),
        }])
    );

    // Official libraries come first, without a description; the shop's matches follow with theirs.
    let libs = world
        .modules
        .services
        .apt
        .search_apt_packages(player, "lib", &[])
        .await
        .unwrap()
        .unwrap();
    let (official, shop) = libs.split_at(libs.len() - 2);

    assert!(!official.is_empty());
    assert!(official.iter().all(|package| package.description.is_none()));
    assert_eq!(
        shop.iter()
            .map(|package| (package.name.as_str(), package.description.is_some()))
            .collect::<Vec<_>>(),
        [("scanlib", true), ("librshell.so", true)]
    );
    assert_eq!(
        world
            .modules
            .services
            .apt
            .install_apt_package(player, ROOT_USER, "scanlib", "/root", &[])
            .await
            .unwrap(),
        Ok(())
    );
    assert_eq!(
        world
            .modules
            .services
            .apt
            .install_apt_package(player, ROOT_USER, "blockchain.so", "/root", &[])
            .await
            .unwrap(),
        Ok(())
    );
    assert_eq!(
        world
            .modules
            .services
            .apt
            .install_apt_package(player, ROOT_USER, "ghostpkg", "/root", &[])
            .await
            .unwrap(),
        Err(AptFailure::UnknownPackage("ghostpkg".to_owned()))
    );

    let program = world
        .modules
        .core
        .files
        .lookup
        .file(player, "/root/scanlib", false)
        .await
        .unwrap()
        .unwrap();
    let library = world
        .modules
        .core
        .files
        .lookup
        .file(player, "/root/blockchain.so", false)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(program.data().unwrap().kind, FileKind::Program);
    assert_eq!(library.data().unwrap().kind, FileKind::Library);
    assert!(
        world
            .modules
            .libraries
            .generator
            .full_library_of(&library)
            .await
            .unwrap()
            .is_some()
    );
}

/// `PlayerUtils.UpdateHackshopLibs` runs when `AptClientIntrinsics.install` reaches a HackShop: a library older than
/// `VersionsControl.lastLibs` is replaced before it is copied.
#[tokio::test]
async fn should_bring_the_libraries_of_a_hackshop_up_to_date_when_installing_from_it() {
    let (world, player, shop) = world_with_shop_source().await;
    let path = "/Public/htdocs/downloads/metaxploit.so";
    let latest = world
        .modules
        .libraries
        .generator
        .latest_full_library(LibraryKind::Metaxploit)
        .await
        .unwrap()
        .unwrap();
    let file = world
        .modules
        .core
        .files
        .lookup
        .file(shop, path, false)
        .await
        .unwrap()
        .unwrap();

    world
        .modules
        .libraries
        .generator
        .set_full_library_of(
            &file,
            Some(&Library {
                kind: LibraryKind::Metaxploit,
                version: "0.1.0".to_owned(),
                zones: Vec::new(),
            }),
        )
        .await
        .unwrap();

    assert_eq!(
        world
            .modules
            .services
            .apt
            .install_apt_package(player, ROOT_USER, "metaxploit.so", "/root", &[])
            .await
            .unwrap(),
        Ok(())
    );

    let installed = world
        .modules
        .core
        .files
        .lookup
        .file(player, "/root/metaxploit.so", false)
        .await
        .unwrap()
        .unwrap();
    let served = world
        .modules
        .core
        .files
        .lookup
        .file(shop, path, false)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        world
            .modules
            .libraries
            .generator
            .full_library_of(&installed)
            .await
            .unwrap(),
        Some(latest.clone())
    );
    assert_eq!(
        world
            .modules
            .libraries
            .generator
            .full_library_of(&served)
            .await
            .unwrap(),
        Some(latest)
    );
    assert_eq!(
        shop_file_names(&world, shop)
            .await
            .last()
            .map(String::as_str),
        Some("metaxploit.so")
    );
}

/// A copied network: its repository computer serves what its own configuration file says, with the packages in the
/// folder it names, as the game's computer it was copied from did.
#[tokio::test]
async fn should_serve_the_catalog_of_a_copied_repository_computer() {
    let mut network = layout();
    let DeviceLayout::Computer(player) = &mut network.nodes[1].device else {
        panic!("the player is a computer");
    };

    online(player);
    put(
        &mut player.filesystem,
        "/etc/apt",
        file(
            "sources.txt",
            FileData::text(&format_apt_sources(&[], true)),
        ),
    );

    let DeviceLayout::Computer(server) = &mut network.nodes[3].device else {
        panic!("the server is a computer");
    };
    let mut library = file(&ServiceId::Repository.library(), FileData::library(1000));

    library.library = Some(Box::new(Library {
        kind: ServiceId::Repository.library_kind(),
        version: "1.0.0".to_owned(),
        zones: Vec::new(),
    }));
    online(server);
    server.ports.push(Port {
        number: 1542,
        service: ServiceId::Repository,
        version: "1.0.0".to_owned(),
        closed: false,
        visible: true,
    });
    put(&mut server.filesystem, "/lib", library);
    put(
        &mut server.filesystem,
        "/server/conf",
        file(
            "repod.conf",
            FileData::text(
                r#"{"path_repo": "/packages", "repo_sources": {"tool": "A copied tool.", "missing": "Not in the folder."}}"#,
            ),
        ),
    );
    put(
        &mut server.filesystem,
        "/packages",
        file("tool", FileData::binary_from_source("print(\"copied\")")),
    );

    let world = World::new(&PlayerOptions {
        network: Some(network),
        ..PlayerOptions::default()
    })
    .await
    .unwrap();
    let player = world.handle.player();

    world
        .modules
        .services
        .apt
        .add_apt_repository(player, "172.16.3.7", 1542)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        world
            .modules
            .services
            .apt
            .update_apt_packages(player, &[])
            .await
            .unwrap(),
        [AptUpdateLine::Update("172.16.3.7".to_owned())]
    );
    assert_eq!(
        world
            .modules
            .services
            .apt
            .show_apt_repository(player, "172.16.3.7", &[])
            .await
            .unwrap(),
        Ok(vec![
            PackageEntry {
                name: "tool".to_owned(),
                description: Some("A copied tool.".to_owned()),
            },
            PackageEntry {
                name: "missing".to_owned(),
                description: Some("Not in the folder.".to_owned()),
            },
        ])
    );
    assert_eq!(
        world
            .modules
            .services
            .apt
            .install_apt_package(player, ROOT_USER, "tool", "/etc/apt", &[])
            .await
            .unwrap(),
        Ok(())
    );
    assert_eq!(
        world
            .modules
            .core
            .files
            .lookup
            .content(player, "/etc/apt/tool", false)
            .await
            .unwrap(),
        Some("print(\"copied\")".to_owned())
    );
    assert_eq!(
        world
            .modules
            .services
            .apt
            .install_apt_package(player, ROOT_USER, "missing", "/etc/apt", &[])
            .await
            .unwrap(),
        Err(AptFailure::MissingPackageFile("missing".to_owned()))
    );
}
