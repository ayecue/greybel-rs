//! Helpers of the library version tests.

use greybel_test_support::future::block_on;
use greyhack_world::{PlayerOptions, World};

/// A world with the default player.
pub fn plain_world() -> World {
    block_on(World::new(&PlayerOptions::default())).unwrap()
}
