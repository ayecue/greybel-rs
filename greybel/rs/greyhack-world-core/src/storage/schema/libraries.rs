//! Library identities shared by files, ports, and router kernels.

use greyhack_world_storage::TableSet;

pub(super) const GROUP: TableSet = TableSet::new(
    "-- one library per kind and version (`VersionsControl`), shared by every file, port and router of that version
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

",
    &["libraries", "library_latest"],
);
