//! Hardware parts and what computers have installed.

use greyhack_world_storage::TableSet;

pub(super) const GROUP: TableSet = TableSet::new(
    "-- hardware parts (`ItemHardware`), one table per kind (`Hardware.TypeHardware`); every computer with a part references
-- its row through `computer_hardware`
CREATE TABLE IF NOT EXISTS hard_disks (
    id             INTEGER PRIMARY KEY,
    name           TEXT NOT NULL,
    quality        INTEGER NOT NULL,
    required_power REAL NOT NULL,
    speed          INTEGER NOT NULL,
    size_mb        REAL NOT NULL,
    UNIQUE (name, quality, required_power, speed, size_mb)
);

CREATE TABLE IF NOT EXISTS cpus (
    id             INTEGER PRIMARY KEY,
    name           TEXT NOT NULL,
    quality        INTEGER NOT NULL,
    required_power REAL NOT NULL,
    speed          REAL NOT NULL,
    cores          INTEGER NOT NULL,
    socket         TEXT NOT NULL,
    UNIQUE (name, quality, required_power, speed, cores, socket)
);

CREATE TABLE IF NOT EXISTS power_supplies (
    id      INTEGER PRIMARY KEY,
    name    TEXT NOT NULL,
    quality INTEGER NOT NULL,
    power   INTEGER NOT NULL,
    UNIQUE (name, quality, power)
);

CREATE TABLE IF NOT EXISTS motherboards (
    id             INTEGER PRIMARY KEY,
    name           TEXT NOT NULL,
    quality        INTEGER NOT NULL,
    required_power REAL NOT NULL,
    cpu_slots      INTEGER NOT NULL,
    cpu_socket     TEXT NOT NULL,
    ram_model      INTEGER NOT NULL,
    ram_slots      INTEGER NOT NULL,
    max_ram_mb     INTEGER NOT NULL,
    pci_slots      INTEGER NOT NULL,
    UNIQUE (name, quality, required_power, cpu_slots, cpu_socket, ram_model, ram_slots, max_ram_mb, pci_slots)
);

CREATE TABLE IF NOT EXISTS rams (
    id             INTEGER PRIMARY KEY,
    name           TEXT NOT NULL,
    quality        INTEGER NOT NULL,
    required_power REAL NOT NULL,
    memory_mb      INTEGER NOT NULL,
    model          INTEGER NOT NULL,
    UNIQUE (name, quality, required_power, memory_mb, model)
);

-- network cards (`Hardware.NetworkDevice`, a PCI card)
CREATE TABLE IF NOT EXISTS net_cards (
    id              INTEGER PRIMARY KEY,
    name            TEXT NOT NULL,
    quality         INTEGER NOT NULL,
    required_power  REAL NOT NULL,
    wifi            INTEGER NOT NULL,
    monitor_support INTEGER NOT NULL,
    UNIQUE (name, quality, required_power, wifi, monitor_support)
);

CREATE TABLE IF NOT EXISTS gpus (
    id             INTEGER PRIMARY KEY,
    name           TEXT NOT NULL,
    quality        INTEGER NOT NULL,
    required_power REAL NOT NULL,
    hashrate       REAL NOT NULL,
    UNIQUE (name, quality, required_power, hashrate)
);

-- a part installed in a computer: `hardware_id` is a row of the table `kind` names. Health is the installed part's;
-- order of installation names network interfaces (eth0, wlan0, …)
CREATE TABLE IF NOT EXISTS computer_hardware (
    id          INTEGER PRIMARY KEY,
    computer_id INTEGER NOT NULL REFERENCES computers(id) ON DELETE CASCADE,
    kind        INTEGER NOT NULL,
    hardware_id INTEGER NOT NULL,
    health      REAL NOT NULL
);
CREATE INDEX IF NOT EXISTS computer_hardware_computer ON computer_hardware (computer_id, kind, id);

-- the state of an installed network card
CREATE TABLE IF NOT EXISTS computer_net_cards (
    computer_hardware_id INTEGER PRIMARY KEY REFERENCES computer_hardware(id) ON DELETE CASCADE,
    active               INTEGER NOT NULL,
    monitor_enabled      INTEGER NOT NULL
);

-- the Wi-Fi networks a computer connected to, oldest first (`ConfigOS.savedNetworks`): the last is the most recent,
-- which the player's computer connects to again when it starts
CREATE TABLE IF NOT EXISTS computer_saved_networks (
    id            INTEGER PRIMARY KEY,
    computer_id   INTEGER NOT NULL REFERENCES computers(id) ON DELETE CASCADE,
    bssid         TEXT NOT NULL,
    essid         TEXT NOT NULL,
    password_hash TEXT NOT NULL REFERENCES passwords(hash),
    public_ip     TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS computer_saved_networks_computer ON computer_saved_networks (computer_id, id);

",
    &[
    "hard_disks",
    "cpus",
    "power_supplies",
    "motherboards",
    "rams",
    "net_cards",
    "gpus",
    "computer_hardware",
    "computer_net_cards",
    "computer_saved_networks",
    ],
);
