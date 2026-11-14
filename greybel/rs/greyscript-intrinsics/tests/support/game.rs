use greybel_interpreter::InterpreterOptions;
use greybel_test_support::greyhack;
use greybel_test_support::interpreter::{constant_clock, with_captured_stdout};
use greybel_test_support::io::PrintLog;
use greyhack_world::prelude::*;
use greyhack_world::{Database, PlayerOptions, World};
use greyhack_world_operations::WorldServer;

use super::world_options;

/// Runs a program counting to five on a generated world the game server writes to an in-memory database; responds
/// with the game server and the lines the program printed.
pub async fn counting_game() -> (WorldServer, PrintLog) {
    let player = PlayerOptions::default();
    let database = Database::in_memory().await.expect("an in-memory database");
    let server = WorldServer::with_database(
        World::generate(&world_options(7, 1))
            .await
            .expect("a generated world"),
        database,
        constant_clock(),
    )
    .await
    .expect("the world is written to the database");
    let (lines, options) = with_captured_stdout(InterpreterOptions::new(constant_clock()));
    let error = greyhack::run_in(
        &server,
        "x = 0\nwhile x < 5\nx = x + 1\nend while\nprint x",
        &player.user,
        true,
        options,
    )
    .await;

    assert_eq!(error, None);

    (server, lines)
}
