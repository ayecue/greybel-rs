//! The devices a generated LAN gets besides computers: cameras, fridges and microwaves (`NetworkLan.GetRandomTypeDevice`).
//! The game draws them from a generator seeded with the network's seed and Greybel from its own, so the draws
//! themselves differ and the tests compare what the rule fixes.

use greybel_test_support::game::Game;

use crate::support::game::{setup_player, test_client};
use crate::support::parity_lan_devices::{KINDS, Rules, game_networks, greybel_networks};

/// The random devices of generated networks follow the game's rule: at most six cameras, one fridge and one microwave
/// in a network, a camera in every network that has an appliance, and no gadgets in residential networks. The games'
/// draws come from `System.Random(seed)` and Greybel's from its own generator, so counts per network differ; the
/// rule's outcome per kind of network must not.
#[tokio::test]
async fn should_match_the_rules_of_the_random_devices_of_generated_networks() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;

    setup_player(&client).await;

    // Each kind uses an independent seed-0 world, so generate those copies concurrently before the serial game fixture
    // work. This preserves the exact networks and rules being compared.
    let expected = std::thread::scope(|scope| {
        let handles: Vec<_> = KINDS
            .iter()
            .map(|(_, kind)| scope.spawn(move || Rules::of(&greybel_networks(*kind))))
            .collect();

        handles
            .into_iter()
            .map(|handle| handle.join().expect("the Greybel networks are generated"))
            .collect::<Vec<_>>()
    });

    for ((game_kind, _), expected) in KINDS.into_iter().zip(expected) {
        let actual = Rules::of(&game_networks(&client, game_kind).await);

        assert_eq!(actual, expected, "{game_kind}");
    }

    client.shutdown();
}
