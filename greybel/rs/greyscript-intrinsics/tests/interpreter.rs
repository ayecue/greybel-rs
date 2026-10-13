//! A program of the game server's world: the game server writes the world to its database as the program ends, and
//! reads it back faithfully.

use duplex_test_support::local::run_local;
use greybel_test_support::world::persistent_fingerprint;
use greyhack_world::World;

use crate::support::game::counting_game;

#[test]
fn should_run_a_program_of_the_game_servers_world() {
    run_local(async {
        let (_server, lines) = counting_game().await;

        assert_eq!(*lines.borrow(), ["5"]);
    });
}

#[test]
fn should_reload_what_the_game_server_wrote_as_the_program_ended() {
    run_local(async {
        let (server, _) = counting_game().await;
        let database = server.database().expect("a database");
        let reloaded = World::load(database).await.expect("reloads");

        let live = server.world().await;

        assert_eq!(
            persistent_fingerprint(&reloaded).await,
            persistent_fingerprint(&live).await
        );
    });
}
