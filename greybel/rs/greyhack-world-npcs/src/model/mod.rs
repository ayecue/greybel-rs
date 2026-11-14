//! NPC, administrator-mail, and SMTP entities and failures.

mod mail;
mod npc;
mod smtp;

pub use greyhack_behaviour::{
    AlarmConfig, CompanyAlign, JobRole, NetworkAdmin, NpcSchedule, WorkPerformance,
};
pub use mail::{Email, MailAccount};
pub use npc::{Npc, NpcEvent};
pub use smtp::SmtpFailure;
