//! Libraries, their exploits, memory zones, and weaknesses.

use super::TableGroup;

pub(super) const GROUP: TableGroup = TableGroup {
    schema: "-- one library per kind and version (`VersionsControl`), shared by every file, port and router of that version
CREATE TABLE IF NOT EXISTS libraries (
    id      INTEGER PRIMARY KEY AUTOINCREMENT,
    kind    INTEGER NOT NULL,
    version TEXT NOT NULL,
    UNIQUE (kind, version)
);

-- the newest version of each library kind (`VersionsControl.lastLibs`), what repositories install
CREATE TABLE IF NOT EXISTS library_latest (
    kind       INTEGER PRIMARY KEY,
    library_id INTEGER NOT NULL REFERENCES libraries(id)
);

-- the exploits the hack shop sells for each library version (`VersionsControl.exploits`): what the game draws at
-- random when it makes one, and whether its weakness was patched, as everything else follows from the library's
-- weaknesses
CREATE TABLE IF NOT EXISTS library_exploits (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    library_id   INTEGER NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
    hidden       INTEGER NOT NULL,
    patched      INTEGER NOT NULL DEFAULT 0,
    zone_address TEXT NOT NULL,
    unsec_value  TEXT NOT NULL,
    name         TEXT NOT NULL,
    part         TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS library_exploits_library ON library_exploits (library_id, id);

-- which of a library version's exploits have been made: those of its visible zones when the version appeared, those
-- of its hidden zones when a newer version replaced it (`VersionsControl.GeneraHideExploit`)
CREATE TABLE IF NOT EXISTS library_exploit_batches (
    library_id INTEGER NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
    hidden     INTEGER NOT NULL,
    PRIMARY KEY (library_id, hidden)
);

CREATE TABLE IF NOT EXISTS memory_zones (
    id         INTEGER PRIMARY KEY,
    library_id INTEGER NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
    address    TEXT NOT NULL,
    hide       INTEGER NOT NULL,
    UNIQUE (library_id, address)
);

CREATE TABLE IF NOT EXISTS weaknesses (
    id                 INTEGER PRIMARY KEY,
    zone_id            INTEGER NOT NULL REFERENCES memory_zones(id) ON DELETE CASCADE,
    remote             INTEGER NOT NULL,
    unsec_value        TEXT NOT NULL,
    details            TEXT NOT NULL,
    reward             INTEGER NOT NULL,
    user               TEXT NOT NULL,
    required_library   INTEGER,
    required_version   TEXT NOT NULL,
    metaxploit_version TEXT NOT NULL,
    num_register_users INTEGER NOT NULL,
    num_port_forward   INTEGER NOT NULL,
    num_conn_gateway   INTEGER NOT NULL,
    path_exist         TEXT NOT NULL,
    random_path        TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS weakness_requirements (
    id          INTEGER PRIMARY KEY,
    weakness_id INTEGER NOT NULL REFERENCES weaknesses(id) ON DELETE CASCADE,
    requirement INTEGER NOT NULL,
    UNIQUE (weakness_id, requirement)
);

",
    tables: &[
    "libraries",
    "library_latest",
    "library_exploits",
    "library_exploit_batches",
    "memory_zones",
    "weaknesses",
    "weakness_requirements",
    ],
};
