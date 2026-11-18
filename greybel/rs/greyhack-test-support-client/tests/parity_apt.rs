//! apt: the official and a custom repository.

use greybel_test_support::game::Game;
use greyhack_behaviour::{APT_CACHE_PATH, APT_SOURCES_PATH, Reward, ServiceId};
use greyhack_test_support_client::protocol::test_support::{
    FileSnapshot, LanNodeKind, NetworkSnapshot,
};

use crate::support::game::{
    fixture_target, hackshop_repository, join_fixture, setup_player, test_client,
};
use crate::support::parity::{assert_succeeded, compare_in, compare_labeled};
use crate::support::remote::script_string;

/// Repository listing, search, source editing, update, upgrade checks and package installation.
#[tokio::test]
async fn should_match_intrinsics_for_apt_repositories() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;

    client
        .install_library(
            "",
            "aptclient.so",
            u8::try_from(Reward::Shell.code()).unwrap(),
        )
        .await
        .unwrap();
    client
        .install_library(
            "",
            "blockchain.so",
            u8::try_from(Reward::Shell.code()).unwrap(),
        )
        .await
        .unwrap();

    let snapshot = client.network_state().await.unwrap();
    let scripts = vec![format!(
        "apt = include_lib(\"/lib/aptclient.so\"); print typeof(apt); print apt.update; print apt.show(\"official_server\"); print apt.show(\"unknown.org\"); print apt.search(\"blockchain\"); print apt.search(\"nope\"); print apt.add_repo(\"\"); print apt.add_repo(\"hackshop.org\"); print apt.add_repo(\"ghost.org\"); home = get_shell.host_computer; print apt.install(\"nmap\", home_dir); print home.File(home_dir + \"/nmap\") == null; print apt.install(\"blockchain.so\", home_dir); f = home.File(home_dir + \"/blockchain.so\"); print f != null; print f.is_binary; print apt.check_upgrade(home_dir + \"/blockchain.so\"); print apt.install(\"ghostpkg\"); print apt.del_repo(\"hackshop.org\"); print apt.del_repo(\"missing.org\"); print apt.search(\"blockchain\"); print apt.add_repo(\"{address}\", 9999); print apt.update",
        address = snapshot.public_ip
    )];

    compare_in(&client, &user, &snapshot, &scripts).await;
    client.shutdown();
}

/// A repository service's own catalog, packages and upgrade comparisons through a custom source. The repository is the
/// HackShop's: `Router.PostGenComputer` replaces its catalog with the files of `/Public/htdocs/downloads`, so the
/// `helloworld` package `Service_repod.ConfigService` gives a repository a player installs is not there, and
/// `AptClientIntrinsics.install` answers "not found in the server" and installs nothing. The script installs what the
/// shop serves: a program (`scanlib`) and a library (`metaxploit.so`).
#[tokio::test]
async fn should_match_intrinsics_for_a_custom_apt_repository() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;

    client
        .install_library(
            "",
            "aptclient.so",
            u8::try_from(Reward::Shell.code()).unwrap(),
        )
        .await
        .unwrap();

    let shop = hackshop_repository(&client).await;
    let target = fixture_target(&shop, "repository");
    let address = target.local_ip.clone();
    let port = target.services[1].internal_port;
    let snapshot = join_fixture(&client, &shop).await;
    let catalog = snapshot
        .nodes
        .iter()
        .filter(|node| node.local_ip == address)
        .find_map(|node| match &node.node {
            LanNodeKind::Computer(computer) => computer
                .files
                .iter()
                .find(|file| file.path == ServiceId::Repository.config_path()),
            _ => None,
        })
        .expect("the repository computer has its catalog")
        .content
        .clone();

    assert!(
        catalog.contains("\"scanlib\"") && catalog.contains("\"metaxploit.so\""),
        "the shop's catalog lacks the packages the script installs: {catalog}"
    );

    let script = format!(
        "apt = include_lib(\"/lib/aptclient.so\"); pc = get_shell.host_computer; print apt.add_repo(\"{address}\", {port}); print apt.update; print apt.show(\"{address}\"); print apt.search(\"scan\"); print apt.install(\"scanlib\", home_dir); file = pc.File(home_dir + \"/scanlib\"); print [file != null, file.is_binary, file.get_content]; print apt.check_upgrade(file.path); print file.set_content(\"older\"); print apt.check_upgrade(file.path); print apt.install(\"metaxploit.so\", home_dir); lib = pc.File(home_dir + \"/metaxploit.so\"); print [lib != null, lib.is_binary]; print apt.check_upgrade(lib.path); print apt.install(\"blockchain.so\", home_dir); print pc.File(home_dir + \"/blockchain.so\") != null; print apt.check_upgrade(home_dir + \"/blockchain.so\")"
    );

    assert_succeeded(&compare_in(&client, &user, &snapshot, &[script]).await);
    client.shutdown();
}

/// The failures of `check_upgrade` in the order of its checks (`AptClientIntrinsics.check_upgrade`): no cache yet, an
/// empty name, a library the official server has and the computer does not, and a name no source has. The no-network
/// refusal comes first of all and needs a computer without a network, which the game fixtures do not make.
#[tokio::test]
async fn should_match_intrinsics_for_the_failures_of_check_upgrade() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;

    client
        .install_library(
            "",
            "aptclient.so",
            u8::try_from(Reward::Shell.code()).unwrap(),
        )
        .await
        .unwrap();

    let snapshot = client.network_state().await.unwrap();
    let script = "apt = include_lib(\"/lib/aptclient.so\"); print apt.check_upgrade(home_dir + \"/blockchain.so\"); print apt.update; print apt.check_upgrade(\"\"); print apt.check_upgrade(home_dir + \"/blockchain.so\"); print apt.check_upgrade(home_dir + \"/ghostpkg.so\")";
    let outcomes = compare_in(&client, &user, &snapshot, &[script.to_owned()]).await;
    let output = outcomes[0].output.as_deref().expect("the script ran");

    assert!(output.contains("aptcache.bin not found"), "{output}");
    assert!(output.contains("package name can not be empty"), "{output}");
    assert!(
        output.contains("blockchain.so does not exist in this filesystem"),
        "{output}"
    );
    assert!(
        output.contains("ghostpkg.so not found in the server"),
        "{output}"
    );
    client.shutdown();
}

fn player_file<'a>(snapshot: &'a NetworkSnapshot, path: &str) -> Option<&'a FileSnapshot> {
    snapshot
        .nodes
        .iter()
        .find_map(|node| match &node.node {
            LanNodeKind::Computer(computer) if computer.player => Some(computer),
            _ => None,
        })
        .expect("the snapshot contains the player computer")
        .files
        .iter()
        .find(|file| file.path == path)
}

/// Local apt cache and source-file corruption, with each failure state isolated by its own snapshot.
#[tokio::test]
async fn should_match_intrinsics_for_local_apt_corruption() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;

    client
        .install_library(
            "",
            "aptclient.so",
            u8::try_from(Reward::Shell.code()).unwrap(),
        )
        .await
        .unwrap();

    for fixture in [
        "apt_cache_missing",
        "apt_cache_binary",
        "apt_cache_malformed",
        "apt_cache_empty",
    ] {
        let snapshot = client.network_state().await.unwrap();
        let prepare = "apt = include_lib(\"/lib/aptclient.so\"); print apt.update".to_owned();

        compare_labeled(
            &client,
            &user,
            &snapshot,
            &format!("{fixture}:prepare-cache"),
            &[prepare],
        )
        .await;

        // Updating and deleting in one script lets the game delete an older cache before the newly written one becomes
        // visible. Separate the two steps and verify both states in the snapshots used for the comparison.
        let prepared = client.network_state().await.unwrap();
        assert!(
            player_file(&prepared, APT_CACHE_PATH).is_some(),
            "{fixture}: apt.update created the cache before the fixture"
        );

        client.configure_computer("", fixture).await.unwrap();
        let snapshot = client.network_state().await.unwrap();
        let cache = player_file(&snapshot, APT_CACHE_PATH);

        match fixture {
            "apt_cache_missing" => assert!(cache.is_none(), "the fixture deleted the new cache"),
            "apt_cache_empty" => assert_eq!(cache.map(|file| file.content.as_str()), Some("")),
            _ => {}
        }
        let script = "apt = include_lib(\"/lib/aptclient.so\"); print apt.check_upgrade(home_dir + \"/blockchain.so\"); print apt.install(\"blockchain.so\", home_dir); print apt.search(\"blockchain\"); print apt.show(\"official_server\")".to_owned();

        compare_labeled(&client, &user, &snapshot, fixture, &[script]).await;
    }

    for fixture in [
        "apt_sources_missing",
        "apt_sources_binary",
        "apt_sources_malformed",
        "apt_sources_unknown",
    ] {
        // Use a fresh player for every sources fixture. The game's delayed file sync can otherwise leave the script on
        // the preceding case's binary file while the mirror already has this case's text.
        let user = setup_player(&client).await;
        client
            .install_library(
                "",
                "aptclient.so",
                u8::try_from(Reward::Shell.code()).unwrap(),
            )
            .await
            .unwrap();

        client.configure_computer("", fixture).await.unwrap();
        let snapshot = client.network_state().await.unwrap();
        let sources = player_file(&snapshot, APT_SOURCES_PATH);

        match fixture {
            "apt_sources_missing" => assert!(sources.is_none()),
            "apt_sources_binary" => {
                assert!(sources.is_some_and(|file| file.binary));
            }
            "apt_sources_malformed" => {
                assert_eq!(sources.map(|file| file.content.as_str()), Some("not json"));
                assert!(sources.is_some_and(|file| !file.binary));
            }
            "apt_sources_unknown" => {
                assert_eq!(
                    sources.map(|file| file.content.as_str()),
                    Some("{\"official_server\":false,\"sourceList\":{\"ghost.org\":0}}")
                );
                assert!(sources.is_some_and(|file| !file.binary));
            }
            _ => unreachable!(),
        }

        let script = if fixture == "apt_sources_unknown" {
            "apt = include_lib(\"/lib/aptclient.so\"); print apt.update".to_owned()
        } else {
            "apt = include_lib(\"/lib/aptclient.so\"); print apt.add_repo(\"1.2.3.4\", 8080); print apt.del_repo(\"1.2.3.4\"); print apt.update".to_owned()
        };

        compare_labeled(&client, &user, &snapshot, fixture, &[script]).await;
    }

    client.shutdown();
}

/// Repository catalog, package and service failures after the player has a valid cache of that source.
#[tokio::test]
async fn should_match_intrinsics_for_repository_corruption() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;

    client
        .install_library(
            "",
            "aptclient.so",
            u8::try_from(Reward::Shell.code()).unwrap(),
        )
        .await
        .unwrap();

    for fixture in [
        "repository_missing_db",
        "repository_binary_db",
        "repository_malformed_db",
        "repository_missing_folder",
        "repository_missing_package",
        "repository_empty_package",
        "repository_malformed_package",
        "repository_service_down",
    ] {
        let shop = hackshop_repository(&client).await;
        let target = fixture_target(&shop, "repository");
        let address = target.local_ip.clone();
        let port = target.services[1].internal_port;
        join_fixture(&client, &shop).await;
        // Writing before creating and joining the fixture lets the game's delayed filesystem synchronization leave an
        // empty snapshot while a later script sees different content. Write only after joining, then use a fresh,
        // verified snapshot for both sides.
        let empty_sources = "{\"official_server\":false,\"sourceList\":{}}";
        client
            .write_file("/etc/apt/sources.txt", empty_sources)
            .await
            .unwrap();
        let snapshot = client.network_state().await.unwrap();
        assert_eq!(
            player_file(&snapshot, "/etc/apt/sources.txt").map(|file| file.content.as_str()),
            Some(empty_sources),
            "{fixture}: reset sources before the valid-cache comparison"
        );
        if let Some(admin) = snapshot.admin.as_ref() {
            client.fire_npc(&shop.public_ip, &admin.user).await.unwrap();
        }

        let valid = format!(
            "apt = include_lib(\"/lib/aptclient.so\"); print apt.add_repo(\"{address}\", {port}); print apt.add_repo(\"{address}\", {port}); print apt.update; print apt.check_upgrade(home_dir + \"/missing.so\")"
        );

        compare_labeled(
            &client,
            &user,
            &snapshot,
            &format!("{fixture}:valid-cache"),
            &[valid],
        )
        .await;

        client.configure_computer(&address, fixture).await.unwrap();
        let snapshot = client.network_state().await.unwrap();

        assert!(
            snapshot
                .admin
                .as_ref()
                .is_none_or(|admin| admin.dead_or_fired),
            "{fixture}"
        );

        let sources = script_string(&format!(
            "{{\"official_server\":false,\"sourceList\":{{\"{address}\":{port}}}}}"
        ));
        let local = if matches!(
            fixture,
            "repository_empty_package" | "repository_malformed_package"
        ) {
            "\"/lib/metaxploit.so\""
        } else {
            "home_dir + \"/metaxploit.so\""
        };
        let script = format!(
            "pc = get_shell.host_computer; pc.touch(\"/etc/apt\", \"sources.txt\"); pc.File(\"/etc/apt/sources.txt\").set_content({sources}); apt = include_lib(\"/lib/aptclient.so\"); print apt.check_upgrade({local}); print apt.install(\"metaxploit.so\", home_dir); print apt.update"
        );

        compare_labeled(&client, &user, &snapshot, fixture, &[script]).await;
    }

    client.shutdown();
}

/// Offline calls and wrong-self calls of every apt method.
#[tokio::test]
async fn should_match_intrinsics_for_apt_offline_and_wrong_self() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;

    client
        .install_library(
            "",
            "aptclient.so",
            u8::try_from(Reward::Shell.code()).unwrap(),
        )
        .await
        .unwrap();
    let network = client.network_state().await.unwrap();

    client
        .disable_network(&network.public_ip, true)
        .await
        .unwrap();

    let snapshot = client.network_state().await.unwrap();
    let script = "apt = include_lib(\"/lib/aptclient.so\"); print apt.check_upgrade(\"x\"); print apt.install(\"x\", home_dir); print apt.add_repo(\"1.2.3.4\"); print apt.del_repo(\"1.2.3.4\"); print apt.search(\"x\"); print apt.show(\"x\"); print apt.update; print [check_upgrade(1, \"x\"), install(1, \"x\", home_dir), add_repo(1, \"x\"), del_repo(1, \"x\"), search(1, \"x\"), show(1, \"x\"), update(1)]".to_owned();

    compare_labeled(
        &client,
        &user,
        &snapshot,
        "apt-offline-wrong-self",
        &[script],
    )
    .await;
    client.shutdown();
}

/// Installation failures for a missing destination, denied destination and full disk.
#[tokio::test]
async fn should_match_intrinsics_for_apt_install_failures() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;

    client
        .install_library(
            "",
            "aptclient.so",
            u8::try_from(Reward::Shell.code()).unwrap(),
        )
        .await
        .unwrap();

    for fixture in ["missing_destination", "home_readonly", "full_disk"] {
        let prepare = "apt = include_lib(\"/lib/aptclient.so\"); print apt.update".to_owned();
        let snapshot = client.network_state().await.unwrap();

        compare_labeled(
            &client,
            &user,
            &snapshot,
            &format!("{fixture}:prepare"),
            &[prepare],
        )
        .await;

        if fixture != "missing_destination" {
            client.configure_computer("", fixture).await.unwrap();
        }
        let snapshot = client.network_state().await.unwrap();
        let destination = if fixture == "missing_destination" {
            "\"/missing\""
        } else {
            "home_dir"
        };
        let script = format!(
            "apt = include_lib(\"/lib/aptclient.so\"); print apt.install(\"blockchain.so\", {destination})"
        );

        compare_labeled(&client, &user, &snapshot, fixture, &[script]).await;
    }

    client.shutdown();
}
