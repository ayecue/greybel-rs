//! Computers, routers and their firewalls, traffic networks, and smart appliances.

use super::TableGroup;

pub(super) const GROUP: TableGroup = TableGroup {
    schema: "-- every device and computer of a network, as the game's LAN tree (`NetworkLan`): the main router has no parent, LAN
-- routers, switches, hubs and computers hang under a device in the order they were added. A computer's gateway is
-- its own (a hub's computers use the router that adopted them), and its router the one it is connected through.
CREATE TABLE IF NOT EXISTS computers (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    kind        INTEGER NOT NULL,
    network_id  INTEGER NOT NULL REFERENCES networks(id) ON DELETE CASCADE,
    parent_id   INTEGER REFERENCES computers(id) ON DELETE CASCADE,
    parent_seq  INTEGER,
    router_id   INTEGER REFERENCES computers(id) ON DELETE SET NULL,
    gateway     TEXT NOT NULL DEFAULT '',
    name        TEXT NOT NULL,
    -- empty for a hub, which has no address
    local_ip    TEXT NOT NULL,
    next_pid    INTEGER NOT NULL,
    reboot_end  INTEGER NOT NULL DEFAULT 0,
    -- whether it can't be reached whatever the time (`LanPcInfo.isHidden`)
    hidden      INTEGER NOT NULL DEFAULT 0,
    -- the NPC whose smartphone it is (`LanPcInfo.npcOwner`), which can't be reached while the NPC isn't at work
    npc_owner   INTEGER REFERENCES persons(id) ON DELETE SET NULL,
    -- the kind of network it was made for (`Computer.tipoRed`), which stays when it joins another network
    network_kind INTEGER NOT NULL,
    UNIQUE (parent_id, parent_seq)
);
CREATE INDEX IF NOT EXISTS computers_network ON computers (network_id, local_ip);
CREATE INDEX IF NOT EXISTS computers_local_ip ON computers (local_ip);
CREATE INDEX IF NOT EXISTS computers_router ON computers (router_id);

-- what only a router or switch has, keyed by its computer; a LAN router without Wi-Fi has no ESSID and BSSID
CREATE TABLE IF NOT EXISTS routers (
    computer_id        INTEGER PRIMARY KEY REFERENCES computers(id) ON DELETE CASCADE,
    essid              TEXT,
    bssid              TEXT,
    wifi_password_hash TEXT NOT NULL REFERENCES passwords(hash),
    firewall_active    INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS routers_wifi ON routers (bssid COLLATE NOCASE, essid);

CREATE TABLE IF NOT EXISTS router_firewall_rules (
    id          INTEGER PRIMARY KEY,
    router_id   INTEGER NOT NULL REFERENCES routers(computer_id) ON DELETE CASCADE,
    allow       INTEGER NOT NULL,
    port        INTEGER,
    source      TEXT,
    destination TEXT
);
CREATE INDEX IF NOT EXISTS router_firewall_rules_router ON router_firewall_rules (router_id, id);

-- police traffic data of a network
-- a network's traffic-camera state (`TrafficNet`); a network without a row has none of it set
CREATE TABLE IF NOT EXISTS traffic_networks (
    network_id     INTEGER PRIMARY KEY REFERENCES networks(id) ON DELETE CASCADE,
    credentials_id INTEGER REFERENCES persons(id) ON DELETE SET NULL,
    override_enabled INTEGER NOT NULL,
    override_used    INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS smart_appliances (
    computer_id          INTEGER PRIMARY KEY REFERENCES computers(id) ON DELETE CASCADE,
    model                TEXT NOT NULL,
    required_power       INTEGER NOT NULL,
    required_temperature INTEGER NOT NULL,
    power                INTEGER NOT NULL,
    temperature          INTEGER NOT NULL,
    alarm                INTEGER NOT NULL,
    override_enabled     INTEGER NOT NULL,
    -- the world second its hardware works again after an accident (`SmartAppConfigOS.brokenUntil`)
    broken_until         INTEGER NOT NULL
);

",
    tables: &[
    "computers",
    "routers",
    "router_firewall_rules",
    "traffic_networks",
    "smart_appliances",
    ],
};
