//! World database schema. All world state is persisted in these rows. `kind`, `service`, `command`,
//! `reward`, `requirement`, and `gender` columns store Rust enum discriminants; flags are 0 or 1.

mod accounts;
mod computer_state;
mod computers;
mod ctf;
mod economy;
mod files;
mod guilds;
mod hardware;
mod libraries;
mod mail;
mod networks;
mod npcs;
mod routines;
mod software;
mod traces;
mod volatile;
mod world;

pub(crate) use volatile::VOLATILE;

/// The `CREATE TABLE` and `CREATE INDEX` statements of one table group and the tables they create.
struct TableGroup {
    schema: &'static str,
    tables: &'static [&'static str],
}

/// Table groups in creation order: parents before dependent rows because foreign keys are enabled.
const GROUPS: [TableGroup; 16] = [
    accounts::GROUP,
    libraries::GROUP,
    networks::GROUP,
    computers::GROUP,
    hardware::GROUP,
    computer_state::GROUP,
    files::GROUP,
    software::GROUP,
    economy::GROUP,
    mail::GROUP,
    ctf::GROUP,
    npcs::GROUP,
    routines::GROUP,
    guilds::GROUP,
    traces::GROUP,
    world::GROUP,
];

/// `CREATE TABLE` and `CREATE INDEX` statements executed when opening a database.
pub(crate) fn schema() -> String {
    GROUPS.iter().map(|group| group.schema).collect()
}

/// Tables in [`schema`] creation order: parents before dependent rows.
pub(crate) fn tables() -> Vec<&'static str> {
    GROUPS
        .iter()
        .flat_map(|group| group.tables)
        .copied()
        .collect()
}
