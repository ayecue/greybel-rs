use std::rc::Rc;

use greybel_test_support::future::block_on;
use greyhack_world::{PlayerOptions, World};
use scheduler::ManualClock;

use crate::support::scripts::run_game_with_io;
use crate::support::semantics::PollStdin;

/// Runs `code` as the admin on a fresh world whose clock passes only as the script waits.
pub fn run(code: &str) -> (String, Option<String>) {
    let world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let clock = Rc::new(ManualClock::advance_on_sleep());
    let (output, error, _) = run_game_with_io(code, world, true, Rc::new(PollStdin), clock);

    (output, error)
}
