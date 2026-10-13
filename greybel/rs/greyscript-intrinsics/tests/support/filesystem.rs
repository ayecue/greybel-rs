use greyhack_world::{PlayerOptions, World};

pub fn world() -> World {
    greybel_test_support::future::block_on(World::new(&PlayerOptions::default())).unwrap()
}
