use greyhack_behaviour::LogEntry;

use super::mission_file::MissionFile;
use super::mission_mail::MissionMail;

/// What `PostGenMission` adds to one computer.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ComputerContent {
    pub owner: String,
    pub files: Vec<MissionFile>,
    pub log: Vec<LogEntry>,
    pub mails: Vec<MissionMail>,
    /// Passwords the decipher database learns (`Database.AddPassword`).
    pub passwords: Vec<String>,
}
