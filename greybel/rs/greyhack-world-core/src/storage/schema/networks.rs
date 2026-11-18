//! Networks and their public addressing.

use greyhack_world_storage::TableSet;

pub(super) const GROUP: TableSet = TableSet::new(
    "-- a network: what everything on it shares, the address it is reached at from outside and its owner
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
    -- the router's home-network marker (`RouterConfigOS.isHomeNetwork`)
    home          INTEGER NOT NULL DEFAULT 0,
    -- its connection speed, one of the ISP's packs (`NetworkLan.netSpeed`)
    net_speed     INTEGER NOT NULL DEFAULT 6,
    -- the address its NPCs' starter mission mail names, set when a mission was placed in it
    -- (`NetworkLan.missionCreated`; `Persona.GenerateMailStarterMission`)
    mission_contact TEXT
);
CREATE UNIQUE INDEX IF NOT EXISTS networks_domain ON networks (domain) WHERE domain <> '';
CREATE INDEX IF NOT EXISTS networks_map ON networks (map_x, map_y);

",
    &["networks"],
);
