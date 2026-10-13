//! Debug libraries and accounts, and package repositories.

use super::TableGroup;

pub(super) const GROUP: TableGroup = TableGroup {
    schema: "-- patch state saved with each library file; its challenge is derived from its library and the world clock
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

CREATE TABLE IF NOT EXISTS repositories (
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
    tables: &[
    "debug_libraries",
    "debug_accounts",
    "repositories",
    "repository_packages",
    ],
};
