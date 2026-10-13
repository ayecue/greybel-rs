//! Restarting the player's computer (`ComputerIntrinsics.reboot`, the game's client starting it again): the saved
//! Wi-Fi network it connects to again, and the safe-mode terminal.

use greybel_test_support::game::Game;
use greyhack_behaviour::SAFE_MODE_TERMINAL;

use crate::support::game::{join_network_where, setup_player, test_client};
use crate::support::parity_restart::{LAUNCH, SETUP, STATE, connect, routers};
use crate::support::restart::{Phase, compare_with_restarts};

/// The network the player connected to last is the one it connects to when its computer starts again: first the main
/// router after the LAN router, then the LAN router, so that a network connected to again counts as the most recent.
/// A restart in safe mode starts the safe-mode terminal and no desktop, which is what connects it.
#[tokio::test]
async fn should_match_intrinsics_for_the_saved_wifi_network_after_a_restart() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let snapshot = join_network_where(&client, "ssh", |snapshot| routers(snapshot).is_some()).await;
    let (main, lan) = routers(&snapshot).expect("two routers with Wi-Fi");
    let (main_ip, lan_ip) = (main.local_ip.clone(), lan.local_ip.clone());
    let (connect_lan, connect_main) = (connect(&lan), connect(&main));
    let phases = [
        Phase::Script(STATE),
        Phase::Script(&connect_lan),
        Phase::Script(&connect_main),
        Phase::Restart { safe_mode: false },
        Phase::Script(STATE),
        Phase::Script(&connect_lan),
        Phase::Restart { safe_mode: false },
        Phase::Script(STATE),
        Phase::Restart { safe_mode: true },
        Phase::Script(STATE),
        Phase::Restart { safe_mode: false },
        Phase::Script(STATE),
    ];

    let outcomes = compare_with_restarts(&client, &user, &snapshot, &phases).await;
    let output = |index: usize| outcomes[index].output.clone().unwrap_or_default();

    // One outcome for each script: the game itself connects back to what it connected to last.
    assert!(output(3).contains(&main_ip), "{}", output(3));
    assert!(!output(3).contains(&lan_ip), "{}", output(3));
    assert!(output(5).contains(&lan_ip), "{}", output(5));
    assert!(output(6).contains(&lan_ip), "{}", output(6));
    assert!(output(6).contains(SAFE_MODE_TERMINAL), "{}", output(6));
    assert!(output(7).contains(&lan_ip), "{}", output(7));
    assert!(!output(7).contains(SAFE_MODE_TERMINAL), "{}", output(7));
    client.shutdown();
}

/// A restart in safe mode starts the protected terminal whose programs launch what they have no execute permission
/// for, but not what is in a folder they have no access to; a normal restart leaves it.
#[tokio::test]
async fn should_match_intrinsics_for_launching_in_the_safe_mode_terminal() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let snapshot = client.network_state().await.unwrap();
    let phases = [
        Phase::Script(SETUP),
        Phase::Script(LAUNCH),
        Phase::Restart { safe_mode: true },
        Phase::Script(LAUNCH),
        Phase::Restart { safe_mode: false },
        Phase::Script(LAUNCH),
    ];

    let outcomes = compare_with_restarts(&client, &user, &snapshot, &phases).await;
    let output = |index: usize| outcomes[index].output.clone().unwrap_or_default();

    assert!(!output(1).contains("ran"), "{}", output(1));
    assert!(output(2).contains("ran"), "{}", output(2));
    assert!(!output(3).contains("ran"), "{}", output(3));
    client.shutdown();
}
