//! Smart appliances, traffic networks, and the persisted traffic cooldown.

use greyhack_world_storage::TableSet;

pub(crate) const GROUP: TableSet = TableSet::new(
    "-- police traffic data of a network
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

-- one row: the world second when police traffic cameras work again after an accident
CREATE TABLE IF NOT EXISTS traffic_state (
    id             INTEGER PRIMARY KEY CHECK (id = 0),
    cooldown_until INTEGER
);
",
    &["traffic_networks", "smart_appliances", "traffic_state"],
);
