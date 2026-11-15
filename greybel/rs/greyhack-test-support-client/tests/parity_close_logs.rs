//! The `close` entries a terminal leaves on the computer it ran on when its connection ends other than by `exit`: its
//! own computer rebooting (`GreyInterpreter.exitBashReboot`), as `MainClass` handles it with
//! `HelperScript.CloseRemotePrograms`. The game's terminals and the simulated network's are compared by output and logs.

use greybel_test_support::game::Game;

use crate::support::game::{join_fixture, setup_player, ssh_network, test_client};
use crate::support::parity_logs::{compare_typed_logs, connect};
use crate::support::remote::fixture_account;

/// A terminal on a remote computer whose bash reboots that computer ends the script that started the terminal as well
/// (`MainClass` takes a second pending VM off on `exitBashReboot`, so the script's state no longer exists) and logs `close` on
/// the rebooted computer (`HelperScript.CloseRemotePrograms`); the script prints nothing after `start_terminal`.
#[tokio::test]
async fn should_match_the_game_when_the_terminals_own_computer_reboots() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let network = ssh_network(&client).await;
    let account = fixture_account(&network, "ssh", "ssh", false);
    let _snapshot = join_fixture(&client, &network).await;
    let remote = connect(
        &account.address,
        account.port,
        "root",
        &account.root_password,
    );
    let rebooting_bash = format!(
        "{remote}\npc = shell.host_computer\npc.touch(\"/root\", \"bash.src\")\npc.File(\"/root/bash.src\").set_content(\"print get_shell.host_computer.reboot\")\nprint shell.build(\"/root/bash.src\", \"/bin\")\nprint shell.start_terminal\nprint \"back\""
    );

    compare_typed_logs(&client, &user, &rebooting_bash, &[]).await;
    client.shutdown();
}
