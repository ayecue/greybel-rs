//! Passwords and persons.

use greyhack_world_storage::TableSet;

pub(super) const PASSWORD: TableSet = TableSet::new(
    "
-- every hash the world ever set, as the game's `Passwords` table; never deleted
CREATE TABLE IF NOT EXISTS passwords (
    hash  TEXT PRIMARY KEY,
    plain TEXT NOT NULL
);
",
    &["passwords"],
);

pub(super) const PERSON: TableSet = TableSet::new(
    "
-- who a user account belongs to (`Persona`)
CREATE TABLE IF NOT EXISTS persons (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    name         TEXT NOT NULL,
    surname      TEXT NOT NULL,
    gender       INTEGER NOT NULL
);

",
    &["persons"],
);
