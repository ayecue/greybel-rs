//! Scheduled routines, subscribers, and coupons.

use super::TableGroup;

pub(super) const GROUP: TableGroup = TableGroup {
    schema: "-- scheduled work; each kind's data is in its own table, whose row a deleted computer, NPC or coin removes: the runner
-- then deletes the routine
CREATE TABLE IF NOT EXISTS routines (
    id   INTEGER PRIMARY KEY AUTOINCREMENT,
    at   INTEGER NOT NULL,
    kind INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS routines_due ON routines (at, id);

CREATE TABLE IF NOT EXISTS routine_service_restarts (
    routine_id  INTEGER PRIMARY KEY REFERENCES routines(id) ON DELETE CASCADE,
    computer_id INTEGER NOT NULL REFERENCES computers(id) ON DELETE CASCADE,
    service     INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS routine_service_restarts_computer ON routine_service_restarts (computer_id);

CREATE TABLE IF NOT EXISTS routine_npc_switches (
    routine_id INTEGER PRIMARY KEY REFERENCES routines(id) ON DELETE CASCADE,
    npc_id     INTEGER NOT NULL UNIQUE REFERENCES npcs(person_id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS routine_mining_rounds (
    routine_id INTEGER PRIMARY KEY REFERENCES routines(id) ON DELETE CASCADE,
    coin_id    INTEGER NOT NULL UNIQUE REFERENCES coins(id) ON DELETE CASCADE
);

-- programs mining a coin through a sub-wallet on a computer (`BlockchainSystem.pcInfo`); volatile at open
CREATE TABLE IF NOT EXISTS mining_subscribers (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    coin_id      INTEGER NOT NULL REFERENCES coins(id) ON DELETE CASCADE,
    subwallet_id INTEGER NOT NULL REFERENCES subwallets(id) ON DELETE CASCADE,
    computer_id  INTEGER NOT NULL REFERENCES computers(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS mining_subscribers_coin ON mining_subscribers (coin_id, id);
CREATE INDEX IF NOT EXISTS mining_subscribers_computer ON mining_subscribers (computer_id);

-- programs sniffing the network of a computer's router (`NetworkLan.SnifferStart`); volatile at open
CREATE TABLE IF NOT EXISTS sniffer_subscribers (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    computer_id INTEGER NOT NULL REFERENCES computers(id) ON DELETE CASCADE,
    router_id   INTEGER REFERENCES routers(computer_id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS sniffer_subscribers_router ON sniffer_subscribers (router_id, id);

-- A script flooding `computer_id` from the public address `origin`, until it stops (`FloodSystem`). Only the first
-- flood of an address on a computer is `counted` towards the computer's reboot.
CREATE TABLE IF NOT EXISTS flood_subscribers (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    computer_id INTEGER NOT NULL REFERENCES computers(id) ON DELETE CASCADE,
    origin      TEXT NOT NULL,
    counted     INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS flood_subscribers_computer ON flood_subscribers (computer_id, origin);

-- the coupons of the player's computer that pay for shop items (`PlayerConfigOS.cupones`); a computer with no row has the
-- ten every new player starts with
CREATE TABLE IF NOT EXISTS player_coupons (
    computer_id INTEGER PRIMARY KEY REFERENCES computers(id) ON DELETE CASCADE,
    coupons     INTEGER NOT NULL
);

",
    tables: &[
    "routines",
    "routine_service_restarts",
    "routine_npc_switches",
    "routine_mining_rounds",
    "mining_subscribers",
    "sniffer_subscribers",
    "flood_subscribers",
    "player_coupons",
    ],
};
