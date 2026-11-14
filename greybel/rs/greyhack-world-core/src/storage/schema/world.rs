//! The world's metadata.

use greyhack_world_storage::TableSet;

pub(super) const GROUP: TableSet = TableSet::new(
    "CREATE TABLE IF NOT EXISTS world_meta (
    id                     INTEGER PRIMARY KEY CHECK (id = 0),
    seed                   INTEGER NOT NULL,
    rng_state              INTEGER NOT NULL,
    -- 0 once a game over deleted the player's computer (`Database.GameOver`)
    player_computer_id     INTEGER NOT NULL,
    player_person_id       INTEGER NOT NULL REFERENCES persons(id),
    elapsed_seconds        INTEGER NOT NULL,
    clock_carry            INTEGER NOT NULL,
    -- `InfoGen.WorldSecret`: what licence plates and the game's other derived values are keyed by.
    world_secret           BLOB NOT NULL
);
",
    &["world_meta"],
);
