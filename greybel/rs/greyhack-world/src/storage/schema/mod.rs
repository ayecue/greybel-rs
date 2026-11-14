//! World database schema. All world state is persisted in these rows. `kind`, `service`, `command`,
//! `reward`, `requirement`, and `gender` columns store Rust enum discriminants; flags are 0 or 1.

pub(crate) mod volatile;

use greyhack_world_storage::Schema;

/// Returns the world's schema: its table sets in creation order, parents before the tables that
/// reference them because foreign keys are enabled. Core precedes every domain; bank and mail links
/// point from their domain tables to core persons.
pub(crate) fn world_schema() -> Schema {
    let mut schema = Schema::new()
        .with_domain(greyhack_world_core::CoreDomain::schema())
        .with_domain(greyhack_world_economy::EconomyModule::schema())
        .with_domain(greyhack_world_npcs::NpcModule::schema())
        .with_domain(greyhack_world_guilds::GuildModule::schema());

    for (name, tables) in [
        (
            "services",
            greyhack_world_services::ServicesModule::schema().tables,
        ),
        (
            "intrusion",
            greyhack_world_intrusion::IntrusionModule::schema().tables,
        ),
        (
            "smart-devices",
            greyhack_world_smart_devices::SmartDevicesModule::schema().tables,
        ),
        (
            "libraries",
            greyhack_world_libraries::LibraryModule::schema().tables,
        ),
        ("shell", Vec::new()),
        ("creation", Vec::new()),
        (
            "player",
            greyhack_world_player::PlayerModule::schema().tables,
        ),
    ] {
        schema = schema.with_domain(greyhack_world_storage::DomainSchema::new(name, tables));
    }

    schema.with(volatile::VOLATILE)
}

#[cfg(test)]
#[path = "tests/mod.rs"]
pub(crate) mod tests;
