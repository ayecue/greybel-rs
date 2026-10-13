//! The world's metadata.

use super::TableGroup;

pub(super) const GROUP: TableGroup = TableGroup {
    schema: "CREATE TABLE IF NOT EXISTS world_meta (
    id                     INTEGER PRIMARY KEY CHECK (id = 0),
    seed                   INTEGER NOT NULL,
    rng_state              INTEGER NOT NULL,
    player_computer_id     INTEGER NOT NULL REFERENCES computers(id),
    player_person_id       INTEGER NOT NULL REFERENCES persons(id),
    elapsed_seconds        INTEGER NOT NULL,
    clock_carry            INTEGER NOT NULL,
    traffic_cooldown_until INTEGER,
    -- `InfoGen.WorldSecret`: what licence plates and the game's other derived values are keyed by.
    world_secret           BLOB NOT NULL
);
",
    tables: &["world_meta"],
};
