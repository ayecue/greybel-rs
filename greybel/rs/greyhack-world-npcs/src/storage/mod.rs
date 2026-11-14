//! NPC repositories and the tables they own.

mod mail;
mod network_admins;
mod npcs;
pub mod schema;

pub(crate) use mail::MailRepository;
pub(crate) use network_admins::NetworkAdminRepository;
pub(crate) use npcs::NpcRepository;
