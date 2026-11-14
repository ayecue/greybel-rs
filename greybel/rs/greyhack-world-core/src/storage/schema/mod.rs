//! The tables of the core. `kind`, `service`, `command`, and `gender` columns store Rust enum
//! discriminants; flags are 0 or 1.
//!
//! Passwords, persons, libraries, networks, computers and their hardware, state, and files form the
//! base tables. Domain tables follow them and may reference them, never the reverse
//! ([`greyhack_world_storage::Schema`] creates tables in registration order, so a table may
//! reference only tables registered before it).

mod accounts;
mod computer_state;
mod computers;
mod files;
mod hardware;
mod libraries;
mod networks;
mod world;

use greyhack_world_storage::TableSet;

/// The password table shared by every account domain.
pub const PASSWORD_TABLES: TableSet = accounts::PASSWORD;
/// Base person identities.
pub const PERSON_TABLES: TableSet = accounts::PERSON;
/// Core tables after passwords and persons, in creation order.
pub const BASE_TABLES: [TableSet; 6] = [
    libraries::GROUP,
    networks::GROUP,
    computers::GROUP,
    hardware::GROUP,
    computer_state::GROUP,
    files::GROUP,
];

/// All core table sets in creation order.
pub const CORE_TABLES: [TableSet; 8] = [
    accounts::PASSWORD,
    accounts::PERSON,
    libraries::GROUP,
    networks::GROUP,
    computers::GROUP,
    hardware::GROUP,
    computer_state::GROUP,
    files::GROUP,
];

/// The `world_meta` table, which references `persons`: register it after the tables whose rows
/// the world stores state for.
pub const WORLD_META_TABLES: TableSet = world::GROUP;

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
