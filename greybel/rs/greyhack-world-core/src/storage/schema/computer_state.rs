//! Users, ports, forwards, processes, and sessions.

use greyhack_world_storage::TableSet;

pub(super) const GROUP: TableSet = TableSet::new(
    "CREATE TABLE IF NOT EXISTS computer_users (
    id            INTEGER PRIMARY KEY,
    computer_id   INTEGER NOT NULL REFERENCES computers(id) ON DELETE CASCADE,
    name          TEXT NOT NULL,
    password_hash TEXT NOT NULL REFERENCES passwords(hash),
    person_id     INTEGER REFERENCES persons(id) ON DELETE SET NULL,
    deletable     INTEGER NOT NULL,
    UNIQUE (computer_id, name)
);
CREATE INDEX IF NOT EXISTS computer_users_password ON computer_users (password_hash);
CREATE INDEX IF NOT EXISTS computer_users_person ON computer_users (person_id);

CREATE TABLE IF NOT EXISTS user_groups (
    id      INTEGER PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES computer_users(id) ON DELETE CASCADE,
    name    TEXT NOT NULL,
    UNIQUE (user_id, name)
);

-- a service's port; its version is its library's
CREATE TABLE IF NOT EXISTS computer_ports (
    id          INTEGER PRIMARY KEY,
    computer_id INTEGER NOT NULL REFERENCES computers(id) ON DELETE CASCADE,
    number      INTEGER NOT NULL,
    service     INTEGER NOT NULL,
    library_id  INTEGER NOT NULL REFERENCES libraries(id),
    closed      INTEGER NOT NULL,
    visible     INTEGER NOT NULL,
    UNIQUE (computer_id, number)
);
CREATE INDEX IF NOT EXISTS computer_ports_service ON computer_ports (computer_id, service);

-- a router's public port forwarded to a device's port
CREATE TABLE IF NOT EXISTS port_forwards (
    router_id INTEGER NOT NULL REFERENCES routers(computer_id) ON DELETE CASCADE,
    number    INTEGER NOT NULL,
    port_id   INTEGER NOT NULL REFERENCES computer_ports(id) ON DELETE CASCADE,
    PRIMARY KEY (router_id, number)
);
CREATE INDEX IF NOT EXISTS port_forwards_port ON port_forwards (port_id);

-- volatile at open, except system processes, service daemons and reverse-shell clients
CREATE TABLE IF NOT EXISTS computer_processes (
    computer_id INTEGER NOT NULL REFERENCES computers(id) ON DELETE CASCADE,
    pid         INTEGER NOT NULL,
    user        TEXT NOT NULL,
    name        TEXT NOT NULL,
    cpu         REAL NOT NULL,
    ram_mb      REAL NOT NULL,
    protected   INTEGER NOT NULL,
    service     INTEGER,
    terminal    INTEGER NOT NULL,
    PRIMARY KEY (computer_id, pid)
);

-- volatile at open, except NPC logins
CREATE TABLE IF NOT EXISTS computer_sessions (
    id          INTEGER PRIMARY KEY,
    computer_id INTEGER NOT NULL REFERENCES computers(id) ON DELETE CASCADE,
    user        TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS computer_sessions_computer ON computer_sessions (computer_id, id);


",
    &[
        "computer_users",
        "user_groups",
        "computer_ports",
        "port_forwards",
        "computer_processes",
        "computer_sessions",
    ],
);
