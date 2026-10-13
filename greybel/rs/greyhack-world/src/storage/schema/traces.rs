//! Active and passive traces.

use super::TableGroup;

pub(super) const GROUP: TableGroup = TableGroup {
    schema: "-- an active trace (`ActiveTraceSystem.ActiveTrace`): the administrator `admin_id` of `computer_id`'s network traces the
-- player, whose program `terminal` caused it, until the world second `ends_at`
CREATE TABLE IF NOT EXISTS active_traces (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    computer_id INTEGER NOT NULL REFERENCES computers(id) ON DELETE CASCADE,
    admin_id    INTEGER NOT NULL REFERENCES persons(id) ON DELETE CASCADE,
    terminal    INTEGER NOT NULL,
    ends_at     INTEGER NOT NULL
);

-- the computers a passive trace goes through, in order (`PassiveTrace.nextRemoteNetID`)
CREATE TABLE IF NOT EXISTS passive_traces (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    computer_id INTEGER NOT NULL UNIQUE REFERENCES computers(id) ON DELETE CASCADE
);

-- the player's strikes (`PlayerHelperServer.gameOverCont`), the generation of the token their log entries carry
-- (`Database.UpdatePlayerTokenTrace`) and how far the passive trace is: `stage` 0 waits to look at the first
-- computer, 1 to read its log, 2 for the next bounce, from `from_ip` to `to_ip` for the connection of `origin_id`
CREATE TABLE IF NOT EXISTS trace_state (
    id               INTEGER PRIMARY KEY CHECK (id = 0),
    strikes          INTEGER NOT NULL DEFAULT 0,
    token_generation INTEGER NOT NULL DEFAULT 0,
    passive_running  INTEGER NOT NULL DEFAULT 0,
    stage            INTEGER NOT NULL DEFAULT 0,
    from_ip          TEXT NOT NULL DEFAULT '',
    to_ip            TEXT NOT NULL DEFAULT '',
    origin_id        INTEGER
);

CREATE TABLE IF NOT EXISTS routine_active_traces (
    routine_id INTEGER PRIMARY KEY REFERENCES routines(id) ON DELETE CASCADE,
    trace_id   INTEGER NOT NULL UNIQUE REFERENCES active_traces(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS routine_passive_traces (
    routine_id INTEGER PRIMARY KEY REFERENCES routines(id) ON DELETE CASCADE
);

",
    tables: &[
    "active_traces",
    "passive_traces",
    "trace_state",
    "routine_active_traces",
    "routine_passive_traces",
    ],
};
