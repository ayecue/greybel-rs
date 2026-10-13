//! Networks, their administrators, and reported library updates.

use super::TableGroup;

pub(super) const GROUP: TableGroup = TableGroup {
    schema: "-- a network: what everything on it shares, the address it is reached at from outside and its owner
CREATE TABLE IF NOT EXISTS networks (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    kind          INTEGER NOT NULL,
    public_ip     TEXT NOT NULL UNIQUE,
    domain        TEXT NOT NULL COLLATE NOCASE,
    contact_name  TEXT,
    contact_mail  TEXT,
    contact_phone TEXT,
    map_x         REAL NOT NULL,
    map_y         REAL NOT NULL,
    -- whether its LAN was generated as the game generates one, which its administrator restores it to
    has_plan      INTEGER NOT NULL DEFAULT 0,
    -- disabled for not having been paid for (`NetworkLan.disabledNetwork`)
    disabled      INTEGER NOT NULL DEFAULT 0,
    -- its connection speed, one of the ISP's packs (`NetworkLan.netSpeed`)
    net_speed     INTEGER NOT NULL DEFAULT 6,
    -- the address its NPCs' starter mission mail names, set when a mission was placed in it
    -- (`NetworkLan.missionCreated`; `Persona.GenerateMailStarterMission`)
    mission_contact TEXT
);
CREATE UNIQUE INDEX IF NOT EXISTS networks_domain ON networks (domain) WHERE domain <> '';

-- a network's administrator (`NetworkLan.adminInfo`), one of its NPCs when it was generated
CREATE TABLE IF NOT EXISTS network_admins (
    network_id       INTEGER PRIMARY KEY REFERENCES networks(id) ON DELETE CASCADE,
    person_id        INTEGER REFERENCES persons(id) ON DELETE SET NULL,
    intelligence     INTEGER NOT NULL,
    pc_experience    INTEGER NOT NULL,
    work_performance INTEGER NOT NULL,
    dead_or_fired    INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS networks_map ON networks (map_x, map_y);

-- the library a network's administrator reported as the newest of its kind (`NetworkLan.reportedLibUpdates`): its
-- computers' library files of that kind take it when they are restored, and computers made after it start with it
CREATE TABLE IF NOT EXISTS network_library_updates (
    network_id INTEGER NOT NULL REFERENCES networks(id) ON DELETE CASCADE,
    kind       INTEGER NOT NULL,
    library_id INTEGER NOT NULL REFERENCES libraries(id),
    PRIMARY KEY (network_id, kind)
);

",
    tables: &[
    "networks",
    "network_admins",
    "network_library_updates",
    ],
};
