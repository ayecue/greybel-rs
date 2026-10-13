//! Worlds with networks to connect to by Wi-Fi, and scripts that connect to them.

use std::rc::Rc;

use greybel_test_support::future::block_on;
use greyhack_world::{NetworkOptions, RouterId, World};
use scheduler::ManualClock;

use super::network::literal;
use super::read_router;
use super::scripts::run_game_with_io;
use super::semantics::{PollStdin, world_with_player_router_at_origin};

/// A world with two networks in range of the player's router, named `alpha` and `bravo`.
pub fn world_with_networks() -> (World, RouterId, RouterId) {
    let mut world = world_with_player_router_at_origin();
    let mut add = |public_ip: &str, essid: &str, password: &str, x: f32| {
        block_on(world.add_network(&NetworkOptions {
            public_ip: Some(public_ip.to_owned()),
            essid: Some(essid.to_owned()),
            wifi_password: Some(password.to_owned()),
            position: Some([x, 0.0]),
            ..NetworkOptions::default()
        }))
        .unwrap()
    };
    let alpha = add("8.8.8.8", "alpha", "pw-a", 0.1);
    let bravo = add("9.9.9.9", "bravo", "pw-b", 0.2);

    (world, alpha, bravo)
}

/// `connect_wifi` on the network `router` has, as GreyScript source.
pub fn connect(world: &World, router: RouterId, password: &str) -> String {
    let router = read_router(world, router);

    format!(
        "print c.connect_wifi(\"wlan0\", {}, {}, {})",
        literal(router.bssid.as_deref().unwrap_or_default()),
        literal(router.essid.as_deref().unwrap_or_default()),
        literal(password),
    )
}

/// Runs `code` as the game does with a clock that moves on `wait`, which the one-second pause between network switches
/// needs.
pub fn run(code: &str, world: World) -> (String, Option<String>, World) {
    let clock = Rc::new(ManualClock::advance_on_sleep());

    run_game_with_io(code, world, true, Rc::new(PollStdin), clock)
}
