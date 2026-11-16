//! Library versions the game releases while it runs (`VersionsControl.CheckUpdatePrograms`): what `aptclient`, the hack
//! shop's downloads and the exploit shop show of a version released, in the game and in a copy of it.

use greybel_test_support::future::block_on;
use greybel_test_support::game::Game;
use greyhack_behaviour::{LibraryKind, Reward, ServiceId};
use greyhack_test_support_client::protocol::test_support::LanNodeKind;
use greyhack_world::World;

use crate::support::game::{
    fixture_target, hackshop_repository, join_fixture, setup_player, test_client,
};
use crate::support::mirror::library;
use crate::support::parity::assert_succeeded;
use crate::support::parity_library_updates::newest_library;
use crate::support::parity_library_versions::{
    AFTER, BEFORE, compare_after_release, compare_with_libraries, copy_with_libraries,
};

/// A new library version appears on the official apt server: `check_upgrade` offers it, `install` gives it, and a
/// library loaded from the new file has its version (`VersionsControl.lastLibs`, `AptClientIntrinsics.check_upgrade`,
/// `AptClientIntrinsics.install`). The game releases it by counting every visible weakness as used up and running its
/// check, the copy by the same rule.
#[tokio::test]
async fn should_match_intrinsics_for_a_library_version_released_in_the_game() {
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

    let newest = newest_library(&client, "libssh.so").await;
    let snapshot = client.network_state().await.unwrap();
    let before =
        compare_with_libraries(&client, &user, &snapshot, &[&newest], &[BEFORE.to_owned()]).await;

    // Read again: the game's file system has the library the first script installed.
    let snapshot = client.network_state().await.unwrap();
    let (released, after) =
        compare_after_release(&client, &user, &snapshot, "libssh.so", &[AFTER.to_owned()]).await;
    let version = library(&released.after).expect("the new library").version;
    let output = after[0].output.as_deref().expect("the script ran");

    assert_ne!(before[0].output, after[0].output);
    assert_ne!(
        library(&released.before).expect("the library").version,
        version
    );
    assert!(output.contains(&version), "{output}");
    client.shutdown();
}

/// The hack shop sells the newest metaxploit, which every release gives a version too (`PlayerUtils.UpdateHackshopLibs`,
/// `VersionsControl.CheckUpdatePrograms`): the repository serves it when a script installs it.
#[tokio::test]
async fn should_match_intrinsics_for_the_metaxploit_a_hack_shop_sells_after_a_release() {
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

    assert!(
        snapshot.nodes.iter().any(|node| matches!(
            &node.node,
            LanNodeKind::Computer(computer)
                if computer
                    .files
                    .iter()
                    .any(|file| file.path == ServiceId::Repository.config_path())
        )),
        "the shop has a repository"
    );

    let script = format!(
        "apt = include_lib(\"/lib/aptclient.so\"); mx = include_lib(\"/lib/metaxploit.so\"); pc = get_shell.host_computer; print apt.add_repo(\"{address}\", {port}); print apt.update; print pc.create_folder(home_dir, \"shop\"); print apt.install(\"metaxploit.so\", home_dir + \"/shop\"); print mx.load(home_dir + \"/shop/metaxploit.so\").version; print apt.check_upgrade(home_dir + \"/shop/metaxploit.so\")"
    );
    let (released, outcomes) =
        compare_after_release(&client, &user, &snapshot, "libssh.so", &[script]).await;
    let version = library(&released.metaxploit_after)
        .expect("the new metaxploit")
        .version;

    assert_succeeded(&outcomes);
    assert!(
        outcomes[0]
            .output
            .as_deref()
            .is_some_and(|output| output.contains(&version)),
        "{outcomes:?} {version}"
    );
    client.shutdown();
}

/// The exploits of a library version the game replaced: those of its visible weaknesses target it only (`= v`), those
/// of its hidden zones appear with the same target, and the new version has an exploit for each visible weakness that
/// targets it and later ones (`VersionsControl.UpdateExploitDesc`, `VersionsControl.GeneraHideExploit`,
/// `VersionsControl.GeneraExploit`). The weaknesses the game and the copy draw differ, so the comparison is of what
/// the rules fix: versions and counts.
#[tokio::test]
async fn should_match_the_game_exploits_of_a_version_it_released() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let snapshot = client.network_state().await.unwrap();
    let released = client
        .release_library_version("libssh.so")
        .await
        .expect("the game releases a version");
    let before = library(&released.before).expect("the library");
    let after = library(&released.after).expect("the new library");
    let metaxploit = library(&released.metaxploit_before).expect("metaxploit");
    let metaxploit_after = library(&released.metaxploit_after).expect("the new metaxploit");

    let game_old = client
        .query_exploits("libssh.so", &before.version)
        .await
        .unwrap()
        .exploits;
    let game_new = client
        .query_exploits("libssh.so", &after.version)
        .await
        .unwrap()
        .exploits;

    let (copy_old, copy_new, copy_version, copy_metaxploit, copy_visible_new) =
        std::thread::scope(|scope| {
            scope
                .spawn(|| {
                    let world: World = copy_with_libraries(
                        &user,
                        &snapshot,
                        &[&released.before, &released.metaxploit_before],
                    );
                    let version = block_on(
                        world
                            .modules
                            .libraries
                            .versions
                            .release_library_version(LibraryKind::Ssh),
                    )
                    .expect("the version is released")
                    .expect("the library exists");
                    let old = block_on(
                        world
                            .modules
                            .libraries
                            .shop
                            .exploit_offers(LibraryKind::Ssh, &before.version),
                    )
                    .expect("the offers are read")
                    .expect("the old version has offers");
                    let new = block_on(
                        world
                            .modules
                            .libraries
                            .shop
                            .exploit_offers(LibraryKind::Ssh, &version),
                    )
                    .expect("the offers are read")
                    .expect("the new version has offers");
                    let metaxploit = block_on(
                        world
                            .modules
                            .libraries
                            .generator
                            .latest_full_library(LibraryKind::Metaxploit),
                    )
                    .expect("metaxploit is read")
                    .expect("metaxploit exists")
                    .version;

                    let visible_new = block_on(
                        world
                            .modules
                            .libraries
                            .generator
                            .latest_full_library(LibraryKind::Ssh),
                    )
                    .expect("the released library is read")
                    .expect("the released library exists")
                    .zones
                    .iter()
                    .filter(|zone| !zone.hide)
                    .map(|zone| zone.weaknesses.len())
                    .sum::<usize>();

                    (old, new, version, metaxploit, visible_new)
                })
                .join()
                .expect("the world runs")
        });

    assert_eq!(copy_version, after.version);
    assert_eq!(copy_metaxploit, metaxploit_after.version);
    assert_ne!(metaxploit.version, metaxploit_after.version);

    // Every visible weakness was used up, so the exploit of each is patched to target the old version only
    // (`VersionsControl.UpdateExploitDesc`), except a password exploit: the game looks for `.overflow("zone", "value")`
    // in the script, and a password script's call has the new password as a third argument. The exploits of the hidden
    // zones target the old version only from the start.
    let hidden_weaknesses = before
        .zones
        .iter()
        .filter(|zone| zone.hide)
        .map(|zone| zone.weaknesses.len())
        .sum::<usize>();
    let visible = before
        .zones
        .iter()
        .filter(|zone| !zone.hide)
        .flat_map(|zone| &zone.weaknesses);
    let visible_weaknesses = visible.clone().count();
    let patchable = visible
        .filter(|weakness| weakness.reward != Reward::ChangePass)
        .count();
    let target = format!("libssh.so = v{}", before.version);
    let unpatched = format!("libssh.so >= v{}", before.version);
    let count_patched = |descriptions: Vec<&str>| {
        descriptions
            .iter()
            .filter(|description| description.contains(&target))
            .count()
    };
    let game_descriptions: Vec<&str> = game_old
        .iter()
        .map(|exploit| exploit.description.as_str())
        .collect();
    let copy_descriptions: Vec<&str> = copy_old
        .iter()
        .map(|offer| offer.description.as_str())
        .collect();

    assert_eq!(
        game_old.iter().filter(|exploit| exploit.patched).count(),
        patchable
    );
    assert_eq!(game_old.len(), visible_weaknesses + hidden_weaknesses);
    assert_eq!(copy_old.len(), game_old.len());
    assert_eq!(
        count_patched(game_descriptions.clone()),
        patchable + hidden_weaknesses
    );
    assert_eq!(
        count_patched(copy_descriptions.clone()),
        patchable + hidden_weaknesses
    );
    assert_eq!(
        game_descriptions
            .iter()
            .filter(|description| description.contains(&unpatched))
            .count(),
        visible_weaknesses - patchable
    );
    assert_eq!(
        copy_descriptions
            .iter()
            .filter(|description| description.contains(&unpatched))
            .count(),
        visible_weaknesses - patchable
    );

    // `GeneraExploit` creates one offer per visible weakness in each world. New visible zones are then drawn independently
    // in the game and the copy, so compare each offer list with the zones of its own released library, not with each other.
    let later = format!("libssh.so >= v{}", after.version);
    let game_visible_new = after
        .zones
        .iter()
        .filter(|zone| !zone.hide)
        .map(|zone| zone.weaknesses.len())
        .sum::<usize>();

    assert_eq!(game_new.len(), game_visible_new, "{game_new:#?}");
    assert_eq!(copy_new.len(), copy_visible_new, "{copy_new:#?}");
    assert!(
        game_new
            .iter()
            .all(|exploit| exploit.description.contains(&later))
    );
    assert!(
        copy_new
            .iter()
            .all(|offer| offer.description.contains(&later))
    );
    client.shutdown();
}
