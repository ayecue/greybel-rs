//! Shared helpers for `greyhack-world` integration tests.

pub mod admin_restore;
pub mod apt;
pub mod bank_payments;
pub mod debug;
pub mod economy_bank;
pub mod economy_market;
pub mod exploit_shop;
pub mod exploits;
pub mod factory_filesystem;
pub mod filesystem;
pub mod flood;
pub mod generation;
pub mod guilds_records;
pub mod intrusion_ddos;
pub mod lan;
pub mod layout;
pub mod library_updates;
pub mod library_versions;
pub mod missions;
pub mod npc_mail;
pub mod npcs_jobs;
pub mod random_content;
pub mod saved_networks;
pub mod schedule;
pub mod search;
pub mod smart_appliance;
pub mod smart_traffic;
pub mod system_log;
pub mod traces_active;
pub mod world;

pub use greybel_test_support::world::{
    change_computer, change_router, computer, computers, has_computer, router, routers, save_copy,
    subwallet, work_ends, world_options,
};
use greyhack_world::World;

/// The ids of the scheduled work that ended since the world's events were last taken.
pub fn ended(world: &mut World) -> Vec<u64> {
    work_ends(world).into_iter().map(|end| end.id).collect()
}
