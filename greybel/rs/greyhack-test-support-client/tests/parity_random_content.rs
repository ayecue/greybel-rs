//! The random files, images and chat logs of generated computers (`Router.GenerateRandomContent`).
//!
//! The game generates a network when it is first reached and so does the simulation, each from its own seed, so
//! the two are not copies of each other. The same rules are checked on both: what a file is called, who owns it,
//! its permissions and size, how a chat log or document is written, how many a computer holds, and which
//! computers hold none.

use greybel_test_support::game::Game;

use crate::support::game::{join_network_where, setup_player, test_client};
use crate::support::parity_random_content::{game_computers, simulated_computers};
use crate::support::random_content::{random_files, violations};

/// Both the game and the simulation give the computers of a network random files that follow the game's rules.
#[tokio::test]
async fn should_follow_the_rules_of_random_content_in_the_game_and_the_simulation() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;

    setup_player(&client).await;

    let snapshot = join_network_where(&client, "ssh", |snapshot| {
        random_files(&game_computers(snapshot)) > 0
    })
    .await;
    let in_game = game_computers(&snapshot);
    let simulated = simulated_computers(12).await;

    assert!(random_files(&simulated) > 0);
    assert_eq!(violations(&in_game), Vec::<String>::new());
    assert_eq!(violations(&simulated), Vec::<String>::new());
    client.shutdown();
}
