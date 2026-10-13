//! System logs: what connections, logins, deletions, net sessions and exploits write to `/var/system.log` on the
//! computers they reach, and the date of the log, in the game and in the simulated network.

use greybel_test_support::game::Game;
use greyhack_behaviour::Reward;

use crate::support::game::{join_network_where, setup_player, test_client};
use crate::support::metaxploit::{computer_library, overflow_script, weakness};
use crate::support::parity_logs::{LOG_DATE, compare_logs, compare_typed_logs, connect};
use crate::support::remote::{
    find_ssh_with_library, forwarded_root, forwarded_service, service_account, ssh_with_library,
};

/// Logins by LAN address and by the public address, with the right and a wrong password, for root and another user:
/// `open` and `shell` on the target and `bounce` on the player's computer and router; a refused login leaves nothing.
#[tokio::test]
async fn should_match_intrinsics_for_connection_logs() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let snapshot = join_network_where(&client, "ssh", |snapshot| {
        find_ssh_with_library(snapshot, "libssh.so").is_some()
            && forwarded_service(snapshot, "ssh").is_some()
    })
    .await;
    let account = service_account(&snapshot, "ssh");
    let (port, forwarded_password) = forwarded_root(&snapshot, "ssh");
    let public_ip = snapshot.public_ip.to_string();
    let scripts = [
        format!(
            "{}\nprint typeof(shell)\n{LOG_DATE}",
            connect(
                &account.address,
                account.port,
                "root",
                &account.root_password
            )
        ),
        format!(
            "print typeof(get_shell.connect_service(\"{}\", {}, \"root\", \"wrong\", \"ssh\"))",
            account.address, account.port
        ),
        format!(
            "{}\nprint typeof(shell)\n{LOG_DATE}",
            connect(
                &account.address,
                account.port,
                &account.user,
                &account.password
            )
        ),
        format!(
            "{}\nprint typeof(shell)\n{LOG_DATE}",
            connect(&public_ip, port, "root", &forwarded_password)
        ),
    ];

    for script in &scripts {
        compare_logs(&client, &user, script).await;
    }

    // A day later the log's date is the new entry's.
    client.advance_clock(86_400).await.unwrap();
    compare_logs(&client, &user, &scripts[0]).await;
    client.shutdown();
}

/// Deleting files and folders on the player's computer and through a remote shell: `remove` with the path and the
/// address the player came from, nothing for a refused deletion, and a new log when the log itself is deleted.
#[tokio::test]
async fn should_match_intrinsics_for_delete_logs() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let snapshot = join_network_where(&client, "ssh", |snapshot| {
        find_ssh_with_library(snapshot, "libssh.so").is_some()
    })
    .await;
    let account = service_account(&snapshot, "ssh");
    let remote = connect(
        &account.address,
        account.port,
        "root",
        &account.root_password,
    );
    let scripts = [
        "pc = get_shell.host_computer\npc.touch(home_dir, \"gone.txt\")\nprint pc.File(home_dir + \"/gone.txt\").delete\nprint pc.File(\"/var/system.log\").date".to_owned(),
        "pc = get_shell.host_computer\npc.create_folder(home_dir, \"dir\")\npc.touch(home_dir + \"/dir\", \"inner.txt\")\nprint pc.File(home_dir + \"/dir\").delete".to_owned(),
        "pc = get_shell.host_computer\nprint pc.File(\"/etc/passwd\").delete".to_owned(),
        format!(
            "{remote}\npc = shell.host_computer\npc.touch(\"/home/guest\", \"gone.txt\")\nprint pc.File(\"/home/guest/gone.txt\").delete\n{LOG_DATE}"
        ),
        format!(
            "{remote}\npc = shell.host_computer\nprint pc.File(\"/var/system.log\").delete\nprint pc.File(\"/var/system.log\") == null\n{LOG_DATE}"
        ),
    ];

    for script in &scripts {
        compare_logs(&client, &user, script).await;
    }

    client.shutdown();
}

/// `net_use` on a service port, on a router, by LAN address and by the public address: `open` with the metaxploit
/// computer's address and the port, nothing for a router.
#[tokio::test]
async fn should_match_intrinsics_for_net_use_logs() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let snapshot = join_network_where(&client, "ssh", |snapshot| {
        find_ssh_with_library(snapshot, "libssh.so").is_some()
            && forwarded_service(snapshot, "ssh").is_some()
    })
    .await;
    let (_, ssh_ip, ssh_port, _) = ssh_with_library(&snapshot, "libssh.so");
    let (public_port, _) = forwarded_root(&snapshot, "ssh");
    let public_ip = snapshot.public_ip.to_string();
    let scripts = [
        format!(
            "mx = include_lib(\"/lib/metaxploit.so\")\nprint typeof(mx.net_use(\"{ssh_ip}\", {ssh_port}))"
        ),
        format!(
            "mx = include_lib(\"/lib/metaxploit.so\")\nprint typeof(mx.net_use(\"{public_ip}\", {public_port}))"
        ),
        format!(
            "mx = include_lib(\"/lib/metaxploit.so\")\nprint typeof(mx.net_use(\"{public_ip}\", 0))"
        ),
        format!(
            "mx = include_lib(\"/lib/metaxploit.so\")\nprint typeof(mx.net_use(\"{ssh_ip}\", 9999))"
        ),
    ];

    for script in &scripts {
        compare_logs(&client, &user, script).await;
    }

    client.shutdown();
}

/// An overflow that gives a shell: `shell` on the target and `bounce` on the metaxploit computer and its router.
/// Overflows that give nothing leave nothing.
#[tokio::test]
async fn should_match_intrinsics_for_exploit_logs() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let snapshot = join_network_where(&client, "ssh", |snapshot| {
        find_ssh_with_library(snapshot, "libssh.so").is_some_and(|(_, ip, _, _)| {
            weakness(&computer_library(snapshot, &ip, "libssh.so"), |weakness| {
                weakness.reward == Reward::Shell && weakness.requirements.is_empty()
            })
            .is_some()
        })
    })
    .await;
    let (_, ssh_ip, ssh_port, _) = ssh_with_library(&snapshot, "libssh.so");
    let library = computer_library(&snapshot, &ssh_ip, "libssh.so");
    let (address, shell_weakness) = weakness(&library, |weakness| {
        weakness.reward == Reward::Shell && weakness.requirements.is_empty()
    })
    .expect("a shell weakness");
    let shell = overflow_script("", Some((&ssh_ip, ssh_port)), address, shell_weakness, "");
    let wrong = overflow_script("", Some((&ssh_ip, ssh_port)), "0xBAD", shell_weakness, "");

    compare_logs(&client, &user, &wrong).await;
    compare_logs(&client, &user, &shell).await;
    client.shutdown();
}

/// A terminal started on a remote shell and left with `exit` logs `close` on that computer with the address of the
/// computer it came from, once for each terminal; a terminal on the player's own computer, and one whose bash runs out
/// of code instead of `exit`, log nothing (`MainClass`, `HelperScript.CloseRemotePrograms`).
#[tokio::test]
async fn should_match_intrinsics_for_terminal_close_logs() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let snapshot = join_network_where(&client, "ssh", |snapshot| {
        find_ssh_with_library(snapshot, "libssh.so").is_some()
    })
    .await;
    let account = service_account(&snapshot, "ssh");
    let remote = connect(
        &account.address,
        account.port,
        "root",
        &account.root_password,
    );
    let exit = ["exit".to_owned()];
    let custom_bash = format!(
        "{remote}\npc = shell.host_computer\npc.touch(\"/root\", \"bash.src\")\npc.File(\"/root/bash.src\").set_content(\"print 1\")\nprint shell.build(\"/root/bash.src\", \"/bin\")\nprint shell.start_terminal\n{LOG_DATE}"
    );
    let cases = [
        (
            format!("{remote}\nprint shell.start_terminal\nprint \"back\"\n{LOG_DATE}"),
            vec!["whoami".to_owned(), "exit".to_owned()],
        ),
        (
            format!(
                "{remote}\nprint shell.start_terminal\nprint shell.start_terminal\nprint \"back\""
            ),
            vec!["exit".to_owned(), "exit".to_owned()],
        ),
        ("print get_shell.start_terminal".to_owned(), exit.to_vec()),
        (custom_bash, Vec::new()),
    ];

    for (script, inputs) in &cases {
        compare_typed_logs(&client, &user, script, inputs).await;
    }

    client.shutdown();
}
