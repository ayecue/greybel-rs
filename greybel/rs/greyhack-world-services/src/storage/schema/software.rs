//! Package repositories.

use greyhack_world_storage::TableSet;

pub(crate) const GROUP: TableSet = TableSet::new(
    "CREATE TABLE IF NOT EXISTS repositories (
    address TEXT PRIMARY KEY
);

-- a repository's package: a library kind, installed at its newest version, or a program
CREATE TABLE IF NOT EXISTS repository_packages (
    id          INTEGER PRIMARY KEY,
    address     TEXT NOT NULL REFERENCES repositories(address) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    description TEXT NOT NULL,
    library     INTEGER,
    UNIQUE (address, name)
);

",
    &["repositories", "repository_packages"],
);
