//! Mail accounts and their messages.

use greybel_test_support::game::Game;

use crate::support::game::{join_network_of_kind_where, setup_player, test_client};
use crate::support::parity::compare_in_real_time;

/// Mail login, summaries, delivery failures, deletion, argument validation and the two-second send cooldown. Mail IDs
/// are used to delete but not printed: `UserMail.RecibirMail` draws them independently in the game and the simulation.
#[tokio::test]
async fn should_match_intrinsics_for_mail_accounts_and_messages() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;

    join_network_of_kind_where(&client, &["MailServices"], "smtp", |snapshot| {
        snapshot.kind == "MailServices" && !snapshot.disabled
    })
    .await;
    client.setup_mail("parity", "mailpass1").await.unwrap();

    let snapshot = client.network_state().await.unwrap();
    let address = snapshot.player_mail.as_ref().unwrap().address.to_string();
    let recipient = snapshot
        .recipient_mail
        .as_ref()
        .unwrap()
        .address
        .to_string();
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
