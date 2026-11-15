//! World database schema. All world state is persisted in these rows. `kind`, `service`, `command`,
//! `reward`, `requirement`, and `gender` columns store Rust enum discriminants; flags are 0 or 1.

use greyhack_world_storage::{DomainSchema, Schema, TableSet};

/// Returns the world's schema: its table sets in creation order, parents before the tables that
/// reference them because foreign keys are enabled. Core precedes every domain; bank and mail links
/// point from their domain tables to core persons.
pub(crate) fn world_schema() -> Schema {
    let domains = [
        greyhack_world_core::CoreDomain::schema(),
        greyhack_world_economy::EconomyModule::schema(),
        greyhack_world_npcs::NpcModule::schema(),
        greyhack_world_guilds::GuildModule::schema(),
        greyhack_world_traces::TracesModule::schema(),
        greyhack_world_services::ServicesModule::schema(),
        greyhack_world_intrusion::IntrusionModule::schema(),
        greyhack_world_smart_devices::SmartDevicesModule::schema(),
        greyhack_world_libraries::LibraryModule::schema(),
        greyhack_world_shell::ShellModule::schema(),
        greyhack_world_creation::CreationModule::schema(),
        greyhack_world_player::PlayerModule::schema(),
    ];
    let mut schema = Schema::new();

    for domain in &domains {
        schema.register_domain(tables_only(domain.clone()));
    }

    // Run the open-time effects in their original order while each domain owns its statements.
    for (domain, set) in [(4, 0), (6, 0), (1, 0), (8, 0), (2, 0), (1, 1)] {
        schema.register(volatile_set(&domains[domain], set));
    }

    schema
}

fn tables_only(mut domain: DomainSchema) -> DomainSchema {
    domain.tables.retain(|set| set.volatile_sql().is_empty());

    domain
}

fn volatile_set(domain: &DomainSchema, index: usize) -> TableSet {
    domain
        .tables
        .iter()
        .filter(|set| !set.volatile_sql().is_empty())
        .nth(index)
        .copied()
        .expect("the domain has its registered open-time cleanup")
}

#[cfg(test)]
#[path = "tests/mod.rs"]
pub(crate) mod tests;
