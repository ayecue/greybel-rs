//! The file tree: nodes, their commands, and their libraries.

use super::TableGroup;

pub(super) const GROUP: TableGroup = TableGroup {
    schema: "CREATE TABLE IF NOT EXISTS nodes (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    computer_id      INTEGER NOT NULL REFERENCES computers(id) ON DELETE CASCADE,
    parent_id        INTEGER REFERENCES nodes(id) ON DELETE CASCADE,
    path             TEXT NOT NULL,
    -- order among the parent's children; a move appends
    seq              INTEGER NOT NULL,
    kind             INTEGER NOT NULL,
    owner            TEXT NOT NULL,
    grp              TEXT NOT NULL,
    permissions      TEXT NOT NULL,
    symlink          TEXT NOT NULL DEFAULT '',
    protected        INTEGER NOT NULL,
    timestamp        INTEGER NOT NULL DEFAULT 0,
    content          TEXT,
    size             INTEGER,
    allow_import     INTEGER,
    encrypt_password TEXT,
    -- a built program's source map as JSON: its sources and mappings `[first, last, source, line]`
    -- (`Archivo.helperImport`)
    source_map       TEXT,
    default_content  INTEGER NOT NULL DEFAULT 0,
    command_string   TEXT NOT NULL DEFAULT '',
    UNIQUE (computer_id, path)
);
CREATE UNIQUE INDEX IF NOT EXISTS nodes_root ON nodes (computer_id) WHERE parent_id IS NULL;
CREATE INDEX IF NOT EXISTS nodes_children ON nodes (parent_id, seq);

-- the application a file starts (`Archivo.comando`)
CREATE TABLE IF NOT EXISTS node_commands (
    node_id INTEGER PRIMARY KEY REFERENCES nodes(id) ON DELETE CASCADE,
    command INTEGER NOT NULL
);

-- a library file's library; many files share one
CREATE TABLE IF NOT EXISTS node_libraries (
    node_id    INTEGER PRIMARY KEY REFERENCES nodes(id) ON DELETE CASCADE,
    library_id INTEGER NOT NULL REFERENCES libraries(id)
);
CREATE INDEX IF NOT EXISTS node_libraries_library ON node_libraries (library_id);

",
    tables: &["nodes", "node_commands", "node_libraries"],
};
