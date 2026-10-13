//! What a computer of a mission is given (`Router.PostGenMission`, `Router.PrepareProceduralObjective`, and the text
//! resolution of `FileItem`, `HelperConversation`, `HelperPDF` and `MissionTextReplacer`).

mod computer;
mod context;
mod functions;
mod mission_file;
mod mission_mail;
mod resolver;

pub(crate) use computer::ComputerContent;
pub(crate) use context::Context;
pub(crate) use functions::{computer_content, router_log, seed_from_ip};
pub(crate) use mission_file::MissionFile;
