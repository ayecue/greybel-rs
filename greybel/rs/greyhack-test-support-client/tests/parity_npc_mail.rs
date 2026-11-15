//! NPC mailboxes: the mails an NPC account is given the first time it is used (`Database.GetMailAccount`,
//! `Persona.GenerateMailsContents`).
//!
//! The game draws the mails with a generator seeded by a new GUID, and the simulation with its own, so the scripts
//! print what the game's code fixes: which logins work, that every mail is of a kind the game writes, from whom,
//! how many of the zero-day kind each account has, that a second login adds nothing and that a mail sent to an
//! account nobody used yet comes after the mails generated for it.

use greybel_test_support::game::Game;

use crate::support::game::{setup_player, test_client};
use crate::support::parity::compare_in_real_time;
use crate::support::parity_npc_mail::{
    COOLDOWN, accounts, addresses, dismissal, join_mail_network, mail_npcs,
};

/// What every NPC mailbox holds after the first login of each account: kinds of mails, senders, stability, reading a
/// conversation, a fired NPC's login, and a mail sent to an account before its first login.
#[tokio::test]
async fn should_match_intrinsics_for_the_mails_of_npc_accounts() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let first = join_mail_network(&client).await;

    client.setup_mail("parity", "mailpass1").await.unwrap();

    let fired = dismissal(&first).expect("an NPC to dismiss").user.clone();

    client.fire_npc(&first.public_ip, &fired).await.unwrap();
    // The game checks a copy of the NPC's state kept with its workstation (`Persona.npcInfo`), which a dismissal does
    // not update (`NetworkLan.FireNpc`): the mail login of a dismissed NPC works until the workstation is next
    // accessed. A player's connection refreshes it (`PlayerUtils.ConnectService`, `Computer.NpcActions`,
    // `Persona.NextCita`), but only on the computer object that call holds, and a script's later calls may get
    // another one (`HelperScript.GetRemoteComputer` caches a single computer), so the bridge refreshes and saves
    // the workstation itself (`Database.SyncConfigOS`), which `mail_login` then reads (`Database.GetMailAccount`,
    // `NetworkLan.GetPersona`). Greybel reads the NPC directly.
    client
        .refresh_npc_account(&first.public_ip, &fired)
        .await
        .unwrap();

    let snapshot = client.network_state().await.unwrap();
    let npcs = mail_npcs(&snapshot);
    let player = snapshot.player_mail.as_ref().unwrap();
    // The subjects of `Persona.GenerateMailsContents`; "something suspicious" is the starter mission mail of a network
    // with a mission (`Persona.GenerateMailStarterMission`). The game draws the mails with a random seed, so only their
    // kinds and senders are compared.
    let script = format!(
        r#"accounts = [{accounts}]
known = [{known}]
subjects = ["coffee during the break?", "I have a break later", "I will be in the break room", "Don't miss this opportunity!", "a discount for you!", "You can't miss these offers!", "About the company", "about the admin", "something suspicious", "RE: stuff"]
isKnown = function(list, address)
  for candidate in list
    if candidate == address then return true
  end for
  return false
end function
print mail_login(known[0], "wrong")
lazy = accounts[accounts.len - 1]
boxes = []
used = []
for account in accounts[:accounts.len - 1]
  box = mail_login(account[0], account[1])
  print typeof(box)
  if typeof(box) == "MetaMail" then
    boxes.push(box)
    used.push(account)
  else
    print box
  end if
end for
counts = []
threadBox = null
threadId = ""
for index in boxes.indexes
  {cooldown}
  summaries = boxes[index].fetch
  spam = 0
  bad = 0
  odd = []
  for summary in summaries
    from = ""
    subject = ""
    id = ""
    for line in summary.split(char(10))
      if id == "" and line.len >= 8 and line[:8] == "MailID: " then id = line[8:]
      if from == "" and line.len >= 6 and line[:6] == "From: " then from = line[6:]
      if subject == "" and line.len >= 9 and line[:9] == "Subject: " then subject = line[9:]
    end for
    ok = subject[:6] == "about "
    for candidate in subjects
      if candidate == subject then ok = true
    end for
    if not ok then
      bad = bad + 1
      odd.push(subject + " / " + from)
    end if
    if from.len >= 9 and from[:9] == "no-reply@" then
      spam = spam + 1
    else if not isKnown(known, from) and subject != "RE: stuff" then
      bad = bad + 1
      odd.push(subject + " / " + from)
    end if
    if subject == "RE: stuff" and threadBox == null then
      threadBox = boxes[index]
      threadId = id
    end if
  end for
  print "spam within three: " + (spam <= 3)
  print "unexpected mails: " + bad
  if odd.len > 0 then print odd
  counts.push(summaries.len)
end for
for index in used.indexes[:3]
  again = mail_login(used[index][0], used[index][1])
  {cooldown}
  print "second login adds nothing: " + (again.fetch.len == counts[index])
end for
if threadId != "" then
  {cooldown}
  parts = threadBox.read([threadId])[0].split(char(10) + char(10))
  print "conversation has several messages: " + (parts.len >= 3)
end if
player = mail_login("{player}", "{player_password}")
{cooldown}
print player.send(lazy[0], "Hello", "From the player")
{cooldown}
lazyBox = mail_login(lazy[0], lazy[1])
print typeof(lazyBox)
{cooldown}
mails = lazyBox.fetch
print "delivered mail comes last: " + (mails[0].indexOf("Subject: Hello") != null)
print mails.len >= 1"#,
        accounts = accounts(&npcs),
        known = addresses(&npcs),
        cooldown = COOLDOWN,
        player = player.address,
        player_password = player.password,
    );

    compare_in_real_time(&client, &user, &snapshot, &[script]).await;
    client.shutdown();
}

/// Aligned NPCs write about their company to their zero-day contact, who has the mail too
/// (`NetworkLan.ConfigZeroDayInfo`, `Persona.GenerateMailZeroDay`).
#[tokio::test]
async fn should_match_intrinsics_for_the_zero_day_mails_of_npc_accounts() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let first = join_mail_network(&client).await;

    client.config_zero_day_info(&first.public_ip).await.unwrap();

    let snapshot = client.network_state().await.unwrap();
    let npcs = mail_npcs(&snapshot);

    assert!(npcs.iter().any(|npc| npc.company_align != 0));

    let script = format!(
        r#"accounts = [{accounts}]
boxes = []
for account in accounts
  box = mail_login(account[0], account[1])
  print typeof(box)
  boxes.push(box)
end for
for box in boxes
  if typeof(box) == "MetaMail" then
    {cooldown}
    count = 0
    for summary in box.fetch
      if summary.indexOf("Subject: About the company") != null then count = count + 1
    end for
    print "zero-day mails: " + count
  end if
end for"#,
        accounts = accounts(&npcs),
        cooldown = COOLDOWN,
    );

    compare_in_real_time(&client, &user, &snapshot, &[script]).await;
    client.shutdown();
}
