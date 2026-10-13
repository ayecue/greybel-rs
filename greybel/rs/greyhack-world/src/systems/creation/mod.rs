//! Seed-driven creation of networks and computers through [`crate::generation`], including lazy
//! network creation on first address access and NPC creation.

pub(crate) mod devices;
pub(crate) mod lan;
pub(crate) mod layout;
pub(crate) mod mission_drafts;
pub(crate) mod missions;
pub(crate) mod network;
pub(crate) mod npcs;
pub(crate) mod restore;
pub(crate) mod routers;
