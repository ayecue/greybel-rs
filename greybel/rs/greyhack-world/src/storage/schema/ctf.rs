//! CTF accounts, events, and players.

use super::TableGroup;

pub(super) const GROUP: TableGroup = TableGroup {
    schema: "CREATE TABLE IF NOT EXISTS ctf_accounts (
    person_id     INTEGER PRIMARY KEY REFERENCES persons(id) ON DELETE CASCADE,
    user          TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL REFERENCES passwords(hash)
);

CREATE TABLE IF NOT EXISTS ctf_events (
    name           TEXT PRIMARY KEY,
    creator_id     INTEGER NOT NULL REFERENCES persons(id) ON DELETE CASCADE,
    user           TEXT NOT NULL,
    password_hash  TEXT NOT NULL REFERENCES passwords(hash),
    description    TEXT NOT NULL,
    public_address TEXT NOT NULL,
    creator_name   TEXT NOT NULL,
    mail_content   TEXT NOT NULL,
    -- rating sums and vote count; 3, 3 and 1 until the first rating (`CTF_Event`)
    quality_rank    INTEGER NOT NULL DEFAULT 3,
    difficulty_rank INTEGER NOT NULL DEFAULT 3,
    num_votes       INTEGER NOT NULL DEFAULT 1,
    is_rated        INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS ctf_events_creator ON ctf_events (creator_id, name);

CREATE TABLE IF NOT EXISTS ctf_players (
    event_name      TEXT NOT NULL REFERENCES ctf_events(name) ON DELETE CASCADE,
    person_id       INTEGER NOT NULL REFERENCES persons(id) ON DELETE CASCADE,
    completed       INTEGER NOT NULL,
    quality_rate    INTEGER NOT NULL DEFAULT 0,
    difficulty_rate INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (event_name, person_id)
);

",
    tables: &["ctf_accounts", "ctf_events", "ctf_players"],
};
