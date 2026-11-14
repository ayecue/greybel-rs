//! Builds procedural missions from the templates (`ProcMissionBuilder.BuildMission`).

mod attachments;
mod beat;
mod build;
mod build_error;
mod items;
mod npcs;
mod pending_service;
mod pending_system_log;
mod preset;
mod runtime_tags;
mod selection;
mod state;

pub use build::build_mission;
