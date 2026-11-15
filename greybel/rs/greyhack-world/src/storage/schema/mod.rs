//! World database schema. All world state is persisted in these rows. `kind`, `service`, `command`,
//! `reward`, `requirement`, and `gender` columns store Rust enum discriminants; flags are 0 or 1.

use greyhack_world_storage::Schema;

/// Returns the world's schema, with each domain's tables and open-time cleanup in dependency order.
pub(crate) fn world_schema() -> Schema {
    let mut schema = Schema::new();

    schema.register_domain(greyhack_world_core::CoreModule::schema());
    schema.register_domain(greyhack_world_economy::EconomyModule::schema());
    schema.register_domain(greyhack_world_npcs::NpcModule::schema());
    schema.register_domain(greyhack_world_guilds::GuildModule::schema());
    schema.register_domain(greyhack_world_traces::TracesModule::schema());
    schema.register_domain(greyhack_world_services::ServicesModule::schema());
    schema.register_domain(greyhack_world_intrusion::IntrusionModule::schema());
    schema.register_domain(greyhack_world_smart_devices::SmartDevicesModule::schema());
    schema.register_domain(greyhack_world_libraries::LibraryModule::schema());
    schema.register_domain(greyhack_world_shell::ShellModule::schema());
    schema.register_domain(greyhack_world_creation::CreationModule::schema());
    schema.register_domain(greyhack_world_player::PlayerModule::schema());

    schema
}

#[cfg(test)]
#[path = "tests/mod.rs"]
pub(crate) mod tests;
