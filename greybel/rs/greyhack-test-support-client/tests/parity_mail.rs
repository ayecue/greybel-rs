//! Mail accounts and their messages.

use greybel_test_support::game::Game;

use crate::support::game::{join_fixture, setup_player, smtp_network, test_client};
use crate::support::parity::{compare_in_real_time, compare_interactive_action, compare_labeled};

/// Mail login, summaries, delivery failures, deletion, argument validation and the two-second send cooldown. Mail IDs
/// are used to delete but not printed: `UserMail.RecibirMail` draws them independently in the game and the simulation.
#[tokio::test]
async fn should_match_intrinsics_for_mail_accounts_and_messages() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let network = smtp_network(&client).await;

    join_fixture(&client, &network).await;
    client.setup_mail("parity", "mailpass1").await.unwrap();

    let snapshot = client.network_state().await.unwrap();
    let address = snapshot.player_mail.as_ref().unwrap().address.clone();
    let recipient = snapshot.recipient_mail.as_ref().unwrap().address.clone();
    let script = format!(
        r#"print mail_login("invalid", "x")
print mail_login("@domain", "x")
print mail_login("name@", "x")
print mail_login("a@b@c", "x")
print mail_login("missing@nowhere", "x")
print mail_login("{address}", "wrong")
mail = mail_login("{address}", "mailpass1")
print typeof(mail)
print mail.fetch
print mail.send("{recipient}", "First", "Hello")
print mail.send("{recipient}", "Second", "Too soon")
print mail.fetch.len
mail = mail_login("{recipient}", "recipientpass")
summaries = mail.fetch
print summaries.len
if summaries.len > 0 then
  lines = summaries[0].split(char(10))
  id = ""
  for line in lines
    if line.len >= 7 and line[:7] == "MailID:" then id = line[8:]
    if line.len < 7 or line[:7] != "MailID:" then print line
  end for
  print mail.delete(id)
  print mail.delete(id)
  print mail.delete(1)
end if
wait(2.1)
print mail.send("missing", "Bad", "Address")
wait(2.1)
print mail.send("{address}", "x" * 129, "Subject")
wait(2.1)
print mail.send("{address}", "Body", "x" * 2501)
wait(2.1)
print mail.send("nobody@nowhere", "Missing", "Recipient")
print mail.send(1, "Bad", "Types")"#
    );

    compare_in_real_time(&client, &user, &snapshot, &[script]).await;
    client.shutdown();
}

/// Long summaries and the argument/error branches of reading and deleting mail.
#[tokio::test]
async fn should_match_intrinsics_for_mail_edges() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let network = smtp_network(&client).await;

    join_fixture(&client, &network).await;
    client.setup_mail(&user, "mailpass").await.unwrap();
    let address = client
        .network_state()
        .await
        .unwrap()
        .player_mail
        .unwrap()
        .address;
    client
        .configure_mail("long_body", &address, "")
        .await
        .unwrap();

    let snapshot = client.network_state().await.unwrap();
    let script = format!(
        "mail = mail_login(\"{address}\", \"mailpass\"); print mail_login(1, 2); messages = mail.fetch; summary = messages[0].split(\"Subject: Long body\\n\")[1]; print messages.len; print summary.len; print summary.slice(summary.len - 4); print mail.read(1); print mail.read([1]); print mail.read([\"missing\"]); print mail.delete([1]); print mail.delete_all; print mail.fetch; print delete_all(1)"
    );

    compare_labeled(&client, &user, &snapshot, "mail-edges", &[script]).await;
    client.shutdown();
}

/// Each offline mail operation reports its own error instead of using the login result.
#[tokio::test]
async fn should_match_intrinsics_for_offline_mail_methods() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let network = smtp_network(&client).await;

    join_fixture(&client, &network).await;
    client.setup_mail(&user, "mailpass").await.unwrap();
    let snapshot = client.network_state().await.unwrap();
    let address = snapshot.player_mail.as_ref().unwrap().address.clone();
    client
        .disable_network(&snapshot.public_ip, true)
        .await
        .unwrap();
    let offline = client.network_state().await.unwrap();
    let script = format!(
        "mail = mail_login(\"{address}\", \"mailpass\"); print mail.fetch; print mail.read([\"missing\"]); print mail.send(\"{address}\", \"x\", \"body\"); print mail.delete([\"missing\"]); print mail.delete_all"
    );

    compare_labeled(&client, &user, &offline, "mail-offline", &[script]).await;
    client.shutdown();
}

/// The recipient blacklist and the three spam-filter modes.
#[tokio::test]
async fn should_match_intrinsics_for_mail_spam_filters() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let network = smtp_network(&client).await;

    join_fixture(&client, &network).await;
    for fixture in ["blacklist", "block_players", "block_npcs", "npc_sender"] {
        client.setup_mail(&user, "mailpass").await.unwrap();
        let current = client.network_state().await.unwrap();
        let sender = current.player_mail.as_ref().unwrap().address.clone();
        let recipient = current.recipient_mail.as_ref().unwrap().address.clone();
        client
            .configure_mail(fixture, &recipient, &sender)
            .await
            .unwrap();
        let snapshot = client.network_state().await.unwrap();
        let script = format!(
            "mail = mail_login(\"{sender}\", \"mailpass\"); print mail.send(\"{recipient}\", \"x\", \"body\")"
        );

        compare_labeled(&client, &user, &snapshot, fixture, &[script]).await;
    }

    client.shutdown();
}

/// Deleting an account while its MetaMail object is alive is observed by `send`.
#[tokio::test]
async fn should_match_intrinsics_for_a_stale_mail_sender() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let network = smtp_network(&client).await;

    join_fixture(&client, &network).await;
    client.setup_mail(&user, "mailpass").await.unwrap();
    let snapshot = client.network_state().await.unwrap();
    let address = snapshot.player_mail.as_ref().unwrap().address.clone();
    let recipient = snapshot.recipient_mail.as_ref().unwrap().address.clone();
    let script = format!(
        "mail = mail_login(\"{address}\", \"mailpass\"); print typeof(mail); user_input(\"continue\"); print mail.send(\"{recipient}\", \"x\", \"body\")"
    );
    let mirror_address = address.clone();
    let mirror_action = std::sync::Arc::new(move |world: &greyhack_world::World| {
        greybel_test_support::future::block_on(
            world.modules.npcs.mail.delete_mail_account(&mirror_address),
        )
        .unwrap();
    });
    let game_address = address.clone();
    let game_client = client.clone();
    let inputs = ["continue".to_owned()];

    compare_interactive_action(
        &client,
        &user,
        &snapshot,
        &script,
        &inputs,
        mirror_action,
        move || {
            let client = game_client.clone();
            let address = game_address.clone();
            async move {
                client
                    .configure_mail("delete_account", &address, "")
                    .await
                    .unwrap();
            }
        },
    )
    .await;
    client.shutdown();
}
