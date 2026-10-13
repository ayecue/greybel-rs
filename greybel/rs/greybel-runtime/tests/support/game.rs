//! Headless Grey Hack test utilities with a player-capable message hook.

use std::path::Path;

use duplex_test_support::local::run_local;
use greybel_test_support::game::Game;

use super::editor::{SourceBreakpoint, Variable, Variables};
use super::runtime::breakpoints_request;
use super::{Harness, MAIN};

/// Starts Grey Hack and its player plugin in `folder` (the test's `CARGO_TARGET_TMPDIR`), then runs `test` with
/// the game port.
pub fn in_game<F: Future<Output = ()>>(folder: impl AsRef<Path>, test: impl FnOnce(u16) -> F) {
    run_local(async {
        let (game, client) = Game::start_with_player(folder).await;

        client.shutdown().await;
        test(game.port()).await;
    });
}

/// Sets breakpoints in [`MAIN`] while driving live game I/O.
pub async fn set_breakpoints_on_io(harness: &mut Harness, breakpoints: Vec<SourceBreakpoint>) {
    harness
        .request_on_io(&breakpoints_request(MAIN, breakpoints))
        .await
        .expect("set");
}

/// Requests variables of the referenced scope or value while driving live game I/O.
pub async fn variables_on_io(harness: &mut Harness, reference: u32) -> Vec<Variable> {
    harness
        .request_on_io(&Variables { reference })
        .await
        .expect("variables")
}
