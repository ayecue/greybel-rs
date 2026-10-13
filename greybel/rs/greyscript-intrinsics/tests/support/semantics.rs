use std::future::ready;

use greybel_interpreter::{InputFuture, Key, Stdin};
use greybel_test_support::future::block_on;
use greyhack_world::{PlayerOptions, World};

use super::{change_router, player_router};

/// A stdin whose every read gives the key `a` at once.
pub struct PollStdin;

impl Stdin for PollStdin {
    fn read_line(&self, _prompt: &str, _echo: greybel_interpreter::Echo) -> InputFuture<String> {
        Box::pin(ready(Ok(String::new())))
    }

    fn read_key(&self, _prompt: &str) -> InputFuture<Key> {
        Box::pin(ready(Ok(Key::Char('a'))))
    }

    fn try_read_key(&self, _prompt: &str) -> InputFuture<Option<Key>> {
        Box::pin(ready(Ok(Some(Key::Char('a')))))
    }
}

/// A new world whose player's router is at the origin of the map.
pub fn world_with_player_router_at_origin() -> World {
    let mut world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let source = player_router(&world);

    change_router(&mut world, source, |router| router.map = [0.0, 0.0]);

    world
}
