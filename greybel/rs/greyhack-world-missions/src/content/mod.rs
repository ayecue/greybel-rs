//! What a computer of a mission is given (`Router.PostGenMission`, `Router.PrepareProceduralObjective`, and the text
//! resolution of `FileItem`, `HelperConversation`, `HelperPDF` and `MissionTextReplacer`).

mod computer;
mod context;
mod functions;
mod mission_file;
mod mission_mail;
mod resolver;

pub use computer::ComputerContent;
pub use context::Context;
pub use functions::{computer_content, router_log, seed_from_ip};
pub use mission_file::MissionFile;
pub use mission_mail::MissionMail;
