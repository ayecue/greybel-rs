//! apt: the official and a custom repository.

use greybel_test_support::game::Game;
use greyhack_behaviour::{Reward, ServiceId};
use greyhack_test_support_client::protocol::test_support::LanNodeKind;

use crate::support::game::{
    fixture_target, hackshop_repository, join_fixture, setup_player, test_client,
};
use crate::support::parity::{assert_succeeded, compare_in};

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
