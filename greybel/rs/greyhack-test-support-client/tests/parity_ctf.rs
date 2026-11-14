//! CTF events and their accounts.

use greybel_test_support::game::Game;

use crate::support::game::{join_network_of_kind_where, setup_player, test_client};
use crate::support::parity::{compare_in, snapshot_outcomes};
use crate::support::remote::script_string;

/// A CTF event created and accepted through the game's mission RPCs: credential failures, all event reads, one
/// completion, and changing the creator's password on both the account and event.
#[tokio::test]
async fn should_match_intrinsics_for_ctf_events() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let event_name = format!("ParityMission{suffix}");

    join_network_of_kind_where(&client, &["MailServices"], "smtp", |snapshot| {
        snapshot.kind == "MailServices" && !snapshot.disabled
    })
    .await;
    client
        .setup_blockchain("Gb8134", &format!("ctfwallet{suffix}"))
        .await
        .unwrap();
    client.setup_mail("ctfplayer", "mailpass").await.unwrap();
    client.setup_ctf(&event_name).await.unwrap();

    let snapshot = client.network_state().await.unwrap();
    let ctf = snapshot.ctf.as_ref().expect("the CTF fixture");
    let event = script_string(&ctf.name);
    let creator = script_string(&ctf.user);
    let script = format!(
        r#"print get_ctf({creator}, "badpass", {event})
print get_ctf({creator}, "oldpass", "MissingMission")
print get_ctf(null, "oldpass", {event})
print get_ctf({creator}, null, {event})
ctf = get_ctf({creator}, "oldpass", {event})
print typeof(ctf)
print ctf.get_description
print ctf.get_template
print ctf.get_creator_name
print ctf.get_mail_content
print ctf.player_success
print ctf.player_success
print reset_ctf_password(null)
print reset_ctf_password("123456789012345678901234567890123")
print reset_ctf_password("bad pass")
print reset_ctf_password("newpass2")
print get_ctf({creator}, "oldpass", {event})
print typeof(get_ctf({creator}, "newpass2", {event}))"#
    );

    compare_in(&client, &user, &snapshot, &[script]).await;

    // `rate_ctf_event` is Greybel's admin-only intrinsic, not a game intrinsic. Exercise the game's own
    // `CTF_Event.AddScore` path through the bridge and compare both results with the intrinsic.
    let completed = client.network_state().await.unwrap();
    let rating = format!(
        r#"admin = include_lib("admin.so")
print admin.rate_ctf_event({event}, 5, 4)
print admin.rate_ctf_event({event}, 5, 4)"#
    );
    let expected = snapshot_outcomes(&[rating.as_str()], &user, &completed).remove(0);

    assert!(!expected.failed, "{expected:?}");

    let mut actual = Vec::new();

    for _ in 0..2 {
        actual.push(client.rate_ctf(&ctf.name, 5, 4).await.unwrap().output);
    }

    assert_eq!(actual.join("\n"), expected.output.unwrap());
    client.shutdown();
}

/// Password validation happens before the game reports that this player has no CTF account.
#[tokio::test]
async fn should_match_intrinsics_for_ctf_password_failures_without_an_account() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let snapshot = client.network_state().await.unwrap();
    let scripts = [r#"print reset_ctf_password(null)
print reset_ctf_password("123456789012345678901234567890123")
print reset_ctf_password("bad pass")
print reset_ctf_password("newpass")
print reset_ctf_password("")"#
        .to_owned()];

    compare_in(&client, &user, &snapshot, &scripts).await;
    client.shutdown();
}
