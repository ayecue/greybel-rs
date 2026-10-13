//! Procedural missions: the templates, the builder that draws a mission from them (`ProcMissionBuilder`), the
//! texts and values the mission's files are made of, and where a mission puts its computers.

mod builder;
mod config;
mod content;
mod dump;
mod model;
mod placement;
mod tags;

pub(crate) use config::Source;
pub(crate) use content::{
    ComputerContent, Context, MissionFile, computer_content, router_log, seed_from_ip,
};
pub use dump::procedural_mission_json;
pub(crate) use model::{ComputerMission, ItemKind, PreGenNpc};
pub(crate) use placement::{
    Assignment, MissionPlacement, MissionRequest, mission_of_group, request_for,
};

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
