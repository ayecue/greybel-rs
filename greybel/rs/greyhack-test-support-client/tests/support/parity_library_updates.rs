//! Helpers of the library update parity tests.

use greybel_test_support::game::Game;
use greyhack_test_support_client::protocol::test_support::NetworkSnapshot;
use greyhack_test_support_client::{GuildSetup, TestClient};

use super::game::{join_new_network_where, setup_player, test_client};
use super::library_updates::{all_older_than, compare_after_report};
use super::parity::Outcome;
use super::parity_guilds::{LEVEL_FIVE, compare_with_guilds};
use super::remote::forwarded_service;

/// The game's content of the newest library file `name`, as a network's administrator reports it. Reported for a network
/// of its own, which the test does not use again.
pub async fn newest_library(client: &TestClient, name: &str) -> String {
    let scratch = client
        .find_new_network("Supermercados", "ssh")
        .await
        .expect("a network is generated");

    client
        .report_library_update(&scratch.public_ip, name)
        .await
        .expect("the game reports the library")
        .content
        .to_string()
}

pub fn ssh_version_script(snapshot: &NetworkSnapshot) -> String {
    let port = forwarded_service(snapshot, "ssh").expect("a forwarded SSH service");

    format!(
        "mx = include_lib(\"/lib/metaxploit.so\")\nprint mx.net_use(\"{}\", {port}).dump_lib.version",
        snapshot.public_ip
    )
}

/// Reports `libssh.so` on a network a guild owns, whose Admin Restore the game's guild has turned off or not
/// (`admin_restore_off`), in the game and in a copy of it. Returns what the SSH version script printed in both worlds
/// before and after, and whether the game's guild had the company on its list of companies with Admin Restore off.
pub async fn report_for_guild(admin_restore_off: bool) -> (Vec<Outcome>, Vec<Outcome>, bool) {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let suffix = std::process::id() % 10000;
    let bank = client
        .setup_blockchain(&format!("Gd{suffix}"), &format!("p{suffix}"))
        .await
        .unwrap()
        .bank_number
        .to_string();
    let newest = newest_library(&client, "libssh.so").await;
    let snapshot = join_new_network_where(&client, "ssh", |snapshot| {
        forwarded_service(snapshot, "ssh").is_some()
            && all_older_than(snapshot, "libssh.so", &newest)
    })
    .await;

    client
        .setup_guild(GuildSetup {
            name: format!("Owls{suffix}"),
            bank_account: bank,
            total_exp: LEVEL_FIVE,
            investment_perks: 1,
            join: true,
            company_ip: snapshot.public_ip.to_string(),
            admin_restore_perks: 1,
            admin_restore_off,
            ..GuildSetup::default()
        })
        .await
        .unwrap();

    let companies = [snapshot.public_ip.to_string()];
    let guilds = client.query_guilds(&companies).await.unwrap();
    let off = guilds.guilds.iter().any(|guild| {
        guild
            .admin_restore_disabled
            .iter()
            .any(|ip| *ip == companies[0])
    });
    let script = ssh_version_script(&snapshot);
    let before = compare_with_guilds(
        &client,
        &user,
        &snapshot,
        &companies,
        std::slice::from_ref(&script),
    )
    .await;
    let after = compare_after_report(
        &client,
        &user,
        &snapshot,
        "libssh.so",
        &newest,
        Some(&guilds),
        std::slice::from_ref(&script),
    )
    .await;

    client.shutdown();

    (before, after, off)
}
