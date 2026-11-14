//! Grey Hack's missions without world state: the mission model (`MissionConfig.Mission` and the items a mission
//! gives its computers), the lists of missions the game authors, the procedural builder (`ProcMissionBuilder`), the
//! texts and files a mission puts on its computers, and which computers of a network it takes.
//!
//! Every kind of mission builds into the same [`Mission`]; [`group_mission`] gives the mission of a group of networks.
//! What changes a world (placing computers in a generated LAN, writing files, accounts and mails into it) is the
//! world's.

mod builder;
mod config;
mod content;
mod dump;
mod group;
mod model;
mod placement;
mod tags;

pub use config::{Archetype, MissionCatalog, ProcMissionsConfig, Source};
pub use content::{
    ComputerContent, Context, MissionFile, MissionMail, computer_content, router_log, seed_from_ip,
};
pub use dump::mission_json;
pub use group::{GroupMission, group_mission, mission_seed, procedural_source};
pub use model::{
    ComputerMission, FileItem, FirewallAction, FirewallItem, FirewallMatch, ItemKind, Mission,
    MissionItem, PreGenNpc, TagMap,
};
pub use placement::{Assignment, MissionPlacement, MissionRequest, assign_computers, request_for};

#[cfg(test)]
#[path = "tests/lib.rs"]
mod tests;
