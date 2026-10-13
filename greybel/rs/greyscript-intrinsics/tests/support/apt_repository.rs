use greybel_test_support::future::block_on;
use greyhack_behaviour::{NetworkKind, kind_for_ip};
use greyhack_world::{NetworkOptions, PlayerOptions, World};

/// A world with a generated HackShop, and the address of its router.
pub fn world_with_hackshop() -> (World, String) {
    let mut world = block_on(World::new(&PlayerOptions::default())).unwrap();
    let address = (1..=u16::MAX)
        .map(|n| format!("8.8.{}.{}", n >> 8, n & 0xff))
        .find(|address| kind_for_ip(world.world_seed(), address) == NetworkKind::HackShop)
        .expect("an address of a HackShop");

    block_on(world.add_network(&NetworkOptions {
        public_ip: Some(address.clone()),
        domain: Some("shop.test".to_owned()),
        computers: Some(1),
        ..NetworkOptions::default()
    }))
    .unwrap();

    (world, address)
}
