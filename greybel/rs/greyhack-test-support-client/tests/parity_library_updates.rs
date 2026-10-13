//! Library updates a network's administrator reports (`Persona.UpdateReportedNetworks`), which the game applies to the
//! network's routers and computers as it restores them (`FileSystem.ApplyReportedLibUpdates`).

use greybel_test_support::game::Game;

use crate::support::game::{join_new_network_where, setup_player, test_client};
use crate::support::library_updates::{all_older_than, compare_after_report, kernel_older_than};
use crate::support::parity::compare_in;
use crate::support::parity_library_updates::{
    newest_library, report_for_guild, ssh_version_script,
};
use crate::support::remote::forwarded_service;

/// The computer a network forwards SSH to has the older libssh.so replaced by the newest the administrator reported,
/// which the script sees on its next connection (`FileSystem.ApplyReportedLibUpdates`, `Database.RestoreComputer`).
#[tokio::test]
async fn should_match_intrinsics_for_a_computer_library_after_a_reported_update() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let newest = newest_library(&client, "libssh.so").await;
    let snapshot = join_new_network_where(&client, "ssh", |snapshot| {
        forwarded_service(snapshot, "ssh").is_some()
            && all_older_than(snapshot, "libssh.so", &newest)
    })
    .await;
    let script = ssh_version_script(&snapshot);
    let before = compare_in(&client, &user, &snapshot, std::slice::from_ref(&script)).await;
    let after = compare_after_report(
        &client,
        &user,
        &snapshot,
        "libssh.so",
        &newest,
        None,
        std::slice::from_ref(&script),
    )
    .await;

    assert_ne!(before[0].output, after[0].output, "{before:?} {after:?}");
    client.shutdown();
}

/// The main router's kernel is replaced by the newest `kernel_router.so` the administrator reported
/// (`Database.RestoreRouter`).
#[tokio::test]
async fn should_match_intrinsics_for_a_router_kernel_after_a_reported_update() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let newest = newest_library(&client, "kernel_router.so").await;
    let snapshot = join_new_network_where(&client, "ssh", |snapshot| {
        kernel_older_than(snapshot, &newest)
    })
    .await;
    let script = format!(
        "print get_router(\"{}\").kernel_version",
        snapshot.public_ip
    );
    let before = compare_in(&client, &user, &snapshot, std::slice::from_ref(&script)).await;
    let after = compare_after_report(
        &client,
        &user,
        &snapshot,
        "kernel_router.so",
        &newest,
        None,
        std::slice::from_ref(&script),
    )
    .await;

    assert_ne!(before[0].output, after[0].output, "{before:?} {after:?}");
    client.shutdown();
}

/// The versions the router lists for the network's ports follow the computers' libssh.so, and the other services keep
/// theirs (`Router.GetPortVersion`).
#[tokio::test]
async fn should_match_intrinsics_for_a_library_the_report_did_not_name() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let newest = newest_library(&client, "libssh.so").await;
    let snapshot = join_new_network_where(&client, "ssh", |snapshot| {
        forwarded_service(snapshot, "ssh").is_some()
            && all_older_than(snapshot, "libssh.so", &newest)
    })
    .await;
    let script = format!(
        "r = get_router(\"{}\")\nfor port in r.used_ports\n  print r.port_info(port)\nend for",
        snapshot.public_ip
    );

    compare_after_report(
        &client,
        &user,
        &snapshot,
        "libssh.so",
        &newest,
        None,
        std::slice::from_ref(&script),
    )
    .await;
    client.shutdown();
}

/// The administrator of a company whose guild turned Admin Restore off does not update the library
/// (`Persona.UpdateReportedNetworks`, `Guilds.IsAdminRestoreOffForCompany`): the network is owned by a guild, with
/// the game's own `Guild.SetAdminRestoreEnabled` turning it off, and the library is reported. The game and the copy
/// both leave the library as it was.
#[tokio::test]
async fn should_match_intrinsics_when_the_guild_turned_admin_restore_off() {
    let (before, after, off) = report_for_guild(true).await;

    assert!(off);
    assert_eq!(before[0].output, after[0].output, "{before:?} {after:?}");
}

/// The same network and guild with Admin Restore left on: the administrator updates the library, in the game and in the
/// copy.
#[tokio::test]
async fn should_match_intrinsics_when_the_guild_left_admin_restore_on() {
    let (before, after, off) = report_for_guild(false).await;

    assert!(!off);
    assert_ne!(before[0].output, after[0].output, "{before:?} {after:?}");
}
