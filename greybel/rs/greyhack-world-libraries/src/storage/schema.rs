//! Library exploits, versions, weaknesses, reports, and reverse shells.

use greyhack_world_storage::TableSet;

pub(crate) const GROUP: TableSet = TableSet::new(
    "-- the exploits the hack shop sells for each library version (`VersionsControl.exploits`): what the game draws at
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

-- the library versions the world keeps per kind, oldest first, which new computers can start with
-- (`VersionsControl.allLibs`)
CREATE TABLE IF NOT EXISTS saved_library_versions (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    kind       INTEGER NOT NULL,
    library_id INTEGER NOT NULL REFERENCES libraries(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS saved_library_versions_kind ON saved_library_versions (kind, id);

-- how often the weaknesses of the newest version of a library kind were used (`Vulnerabilidad.helperHackResult.vecesUsado`
-- in `VersionsControl.lastLibs`); not saved with the world, so volatile at open
CREATE TABLE IF NOT EXISTS library_weakness_uses (
    kind        INTEGER NOT NULL,
    address     TEXT NOT NULL,
    unsec_value TEXT NOT NULL,
    uses        INTEGER NOT NULL,
    PRIMARY KEY (kind, address, unsec_value)
);

-- how often the weaknesses of a memory zone of the newest version of a library kind were renewed
-- (`Library.MemoryZone.timesPatched`)
CREATE TABLE IF NOT EXISTS library_zone_patches (
    kind          INTEGER NOT NULL,
    address       TEXT NOT NULL,
    times_patched INTEGER NOT NULL,
    PRIMARY KEY (kind, address)
);

-- the players whose report of a weakness the next check for library versions has not processed yet
-- (`VersionsControl.playerIds`); not saved with the world, so volatile at open
CREATE TABLE IF NOT EXISTS library_reporters (
    player INTEGER PRIMARY KEY
);

-- a reverse shell its client process keeps open to a router
CREATE TABLE IF NOT EXISTS reverse_shells (
    id          INTEGER PRIMARY KEY,
    router_id   INTEGER NOT NULL REFERENCES routers(computer_id) ON DELETE CASCADE,
    computer_id INTEGER NOT NULL,
    pid         INTEGER NOT NULL,
    port        INTEGER NOT NULL,
    user        TEXT NOT NULL,
    created_at  INTEGER NOT NULL,
    FOREIGN KEY (computer_id, pid) REFERENCES computer_processes(computer_id, pid) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS reverse_shells_router ON reverse_shells (router_id, id);

-- the library a network's administrator reported as the newest of its kind (`NetworkLan.reportedLibUpdates`): its
-- computers' library files of that kind take it when they are restored, and computers made after it start with it
CREATE TABLE IF NOT EXISTS network_library_updates (
    network_id INTEGER NOT NULL REFERENCES networks(id) ON DELETE CASCADE,
    kind       INTEGER NOT NULL,
    library_id INTEGER NOT NULL REFERENCES libraries(id),
    PRIMARY KEY (network_id, kind)
);

-- patch state saved with each library file; its challenge is derived from its library and the world clock
CREATE TABLE IF NOT EXISTS debug_libraries (
    node_id       INTEGER PRIMARY KEY REFERENCES nodes(id) ON DELETE CASCADE,
    patch_seed    INTEGER,
    auto_patch_at INTEGER,
    num_patches   INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS debug_accounts (
    user             TEXT PRIMARY KEY,
    password_hash    TEXT NOT NULL REFERENCES passwords(hash),
    failed_logins    INTEGER NOT NULL,
    unit_test_failed INTEGER NOT NULL,
    last_network_ip  TEXT
);

",
    &[
        "library_exploits",
        "library_exploit_batches",
        "memory_zones",
        "weaknesses",
        "weakness_requirements",
        "saved_library_versions",
        "library_weakness_uses",
        "library_zone_patches",
        "library_reporters",
        "reverse_shells",
        "network_library_updates",
        "debug_libraries",
        "debug_accounts",
    ],
);

/// Player coupons used by the library shop.
pub(crate) const COUPONS: TableSet = TableSet::new(
    "-- the coupons of the player's computer that pay for shop items (`PlayerConfigOS.cupones`); a computer with no row has the
-- ten every new player starts with
CREATE TABLE IF NOT EXISTS player_coupons (
    computer_id INTEGER PRIMARY KEY REFERENCES computers(id) ON DELETE CASCADE,
    coupons     INTEGER NOT NULL
);
",
    &["player_coupons"],
);

/// The data of library-check routines.
pub(crate) const LIBRARY_CHECKS: TableSet = TableSet::new(
    "CREATE TABLE IF NOT EXISTS routine_library_checks (
    routine_id INTEGER PRIMARY KEY REFERENCES routines(id) ON DELETE CASCADE
);

",
    &["routine_library_checks"],
);

/// Clears weakness-use state and script processes, retaining reverse-shell clients.
pub(crate) const VOLATILE: TableSet = TableSet::volatile(
    "DELETE FROM library_weakness_uses;
DELETE FROM library_reporters;
DELETE FROM computer_processes
WHERE service IS NULL
  AND protected = 0
  AND NOT EXISTS (
      SELECT 1 FROM reverse_shells
      WHERE reverse_shells.computer_id = computer_processes.computer_id AND reverse_shells.pid = computer_processes.pid
  );
",
);
