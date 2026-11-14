//! Sniffer and flood tables and their program-lifetime cleanup.

use greyhack_world_storage::TableSet;

/// Sniffer and flood subscribers.
pub(crate) const GROUP: TableSet = TableSet::new(
    "-- programs sniffing the network of a computer's router (`NetworkLan.SnifferStart`); volatile at open
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
",
    &["sniffer_subscribers", "flood_subscribers"],
);

/// Deletes subscribers whose programs no longer exist when a database opens.
pub(crate) const VOLATILE: TableSet = TableSet::volatile(
    "DELETE FROM sniffer_subscribers;
DELETE FROM flood_subscribers;
",
);
