//! Traces: what the game's active and passive traces do after a script logged in to a company's computer and deleted a
//! file there, with and without the player owning the company (`ActiveTraceSystem.StartActiveTrace`,
//! `PassiveTrace.AddPassiveTrace`, `ActiveTraceSystem.IsTraceProtectedByCompanyOwnership`).

use greybel_test_support::game::Game;
use greyhack_test_support_client::GuildSetup;

use crate::support::game::{join_fixture, setup_player, ssh_network_of_kind, test_client};
use crate::support::parity_game_over::traces_while_waiting;
use crate::support::parity_traces::{LEVEL_FIVE, compare, script};
use crate::support::remote::fixture_account;

/// A script logs in to a bank's computer and deletes a file: the computer is queued for the passive trace and no active
/// trace outlives the script.
#[tokio::test]
async fn should_match_the_game_when_a_script_deletes_a_file_on_a_company_computer() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let network = ssh_network_of_kind(&client, "Bancos").await;
    let account = fixture_account(&network, "ssh", "ssh", false);
    let snapshot = join_fixture(&client, &network).await;

    let traced = compare(&client, &user, &snapshot, &account).await;

    assert_eq!(
        traced.1.len(),
        1,
        "the computer is queued for the passive trace"
    );
    assert!(traced.0.is_empty());

    client.shutdown();
}

/// The same script when the player's guild owns the bank: nothing is traced, neither actively nor passively
/// (`ActiveTraceSystem.IsTraceProtectedByCompanyOwnership`).
#[tokio::test]
async fn should_match_the_game_when_the_players_guild_owns_the_company() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let suffix = std::process::id() % 10000;
    let bank = client
        .setup_blockchain(&format!("Gu{suffix}"), &format!("u{suffix}"))
        .await
        .unwrap()
        .bank_number
        .clone();
    let network = ssh_network_of_kind(&client, "Bancos").await;
    let account = fixture_account(&network, "ssh", "ssh", false);
    let snapshot = join_fixture(&client, &network).await;

    client
        .setup_guild(GuildSetup {
            name: format!("Owls{suffix}"),
            bank_account: bank,
            total_exp: LEVEL_FIVE,
            investment_perks: 1,
            join: true,
            company_ip: snapshot.public_ip.clone(),
            crown: false,
            shield_ip: String::new(),
            admin_restore_perks: 0,
            admin_restore_off: false,
        })
        .await
        .unwrap();

    let traced = compare(&client, &user, &snapshot, &account).await;

    assert!(traced.0.is_empty());
    assert!(traced.1.is_empty());
    client.shutdown();
}

/// A script that logged in to a company's computer and deletes a file there while its terminal is on the player's own
/// computer starts no active trace: only a deletion on the computer of the script's last shell does
/// (`HelperScript.AddActiveTraceFromAction`). Both worlds are read while the script waits.
#[tokio::test]
async fn should_match_the_game_when_a_script_deletes_a_file_on_a_computer_that_is_not_its_last_shell()
 {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let network = ssh_network_of_kind(&client, "Bancos").await;
    let account = fixture_account(&network, "ssh", "ssh", false);
    let snapshot = join_fixture(&client, &network).await;
    let guilds = client
        .query_guilds(std::slice::from_ref(&snapshot.public_ip.clone()))
        .await
        .expect("the guilds are read");
    let script = format!("{}\nuser_input(\"wait\")", script(&account));

    let (actual, expected) =
        traces_while_waiting(&client, &user, &snapshot, &guilds, &script).await;

    assert_eq!(actual, expected);
    assert!(actual.0.is_empty(), "no active trace");
    client.shutdown();
}
