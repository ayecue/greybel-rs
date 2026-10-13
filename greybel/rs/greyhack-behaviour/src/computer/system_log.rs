//! The system log of a computer (`LogSystem`): the JSON in `/var/system.log` that records connections, logins,
//! deletions and exploits.

use serde::{Deserialize, Serialize};

use crate::{hash_password, is_reserved_ip, is_valid_ip};

/// Path of the system log (`PlayerUtils.WriteLog`).
pub const SYSTEM_LOG_PATH: &str = "/var/system.log";

/// Folder of the system log, created with the log when it is missing (`PlayerUtils.WriteLog`).
pub const SYSTEM_LOG_FOLDER: &str = "/var";

/// Name of the system log file.
pub const SYSTEM_LOG_NAME: &str = "system.log";

/// Permissions of a system log that `PlayerUtils.WriteLog` creates.
pub const SYSTEM_LOG_PERMISSIONS: &str = "-rwxr-x---";

/// Permissions of the `/var` folder that `PlayerUtils.WriteLog` creates.
pub const SYSTEM_LOG_FOLDER_PERMISSIONS: &str = "drwxr-xr-x";

/// ID the system log entries name the player by (`GreyInterpreter.GetPlayerID`, the MD5 of the player's Steam ID
/// in the game). The world has one player and no Steam ID, so it derives the ID from its `seed`.
#[must_use]
pub fn player_net_id(seed: u64) -> String {
    hash_password(&format!("player:{seed}"))
}

/// Trace token the system log entries carry after `generation` game overs by passive traces
/// (`Database.GetPlayerTokenTrace`, `Database.UpdatePlayerTokenTrace`): a GUID the game draws when it creates the
/// player, derived from the world `seed` in the same way as [`player_net_id`].
#[must_use]
pub fn token_trace(seed: u64, generation: u32) -> String {
    let digest = if generation == 0 {
        hash_password(&format!("token-trace:{seed}"))
    } else {
        hash_password(&format!("token-trace:{seed}:{generation}"))
    };

    format!(
        "{}-{}-{}-{}-{}",
        &digest[0..8],
        &digest[8..12],
        &digest[12..16],
        &digest[16..20],
        &digest[20..32]
    )
}

/// Number of entries a system log retains (`LogSystem.AddLog`); older entries drop out.
pub const MAX_LOG_ENTRIES: usize = 275;

/// The address a shell's player came from (`PlayerUtils.ConnectService`): the address of the computer the player
/// connects from (`origin`), unless the connection goes to that address, which retains the previous address of the
/// shell it connects from (`previous`).
#[must_use]
pub fn previous_address(origin: &str, address: &str, previous: &str) -> String {
    if address == origin {
        previous.to_owned()
    } else {
        origin.to_owned()
    }
}

enum_def! {
    /// What a log entry records (`LogSystem.StatusLog`).
    #[derive(Serialize, Deserialize)]
    #[serde(into = "i64", try_from = "i64")]
    pub enum LogAction {
        Open = 0,
        Close = 1,
        Shell = 2,
        Remove = 3,
        Bounce = 4,
        Download = 5,
        Upload = 6,
    }
}

/// One log entry (`LogSystem.Content`), serialized with the game's field names and order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogEntry {
    pub action: LogAction,
    pub ip: String,
    /// Game date in `dd/MMM/yyyy - HH:mm` format.
    pub fecha: String,
    pub puerto: i64,
    pub file: String,
    #[serde(rename = "bounceIp")]
    pub bounce_ip: String,
    #[serde(rename = "playerNetID")]
    pub player_net_id: String,
    pub tutorial: bool,
    #[serde(rename = "tokenTrace", default)]
    pub token_trace: String,
}

/// The entries of a system log (`LogSystem`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemLog {
    #[serde(rename = "contentLog")]
    pub content_log: Vec<LogEntry>,
}

impl SystemLog {
    /// Reads the log in a file's content: an empty content is an empty log, and content that is not a log yields
    /// `None` (`JsonConvert.DeserializeObject<LogSystem>`).
    #[must_use]
    pub fn parse(content: &str) -> Option<Self> {
        if content.is_empty() {
            return Some(Self::default());
        }

        serde_json::from_str(content).ok()
    }

    /// Appends `entry` and drops the oldest entries beyond [`MAX_LOG_ENTRIES`] (`LogSystem.AddLog`).
    pub fn add(&mut self, entry: LogEntry) {
        self.content_log.push(entry);

        if self.content_log.len() > MAX_LOG_ENTRIES {
            let excess = self.content_log.len() - MAX_LOG_ENTRIES;

            self.content_log.drain(..excess);
        }
    }

    /// The first entry that recorded a connection bounced to `bounced_ip` for the player `player_net_id` under the trace
    /// token `token_trace` (`LogSystem.GetIniBounce`).
    #[must_use]
    pub fn ini_bounce(
        &self,
        bounced_ip: &str,
        player_net_id: &str,
        token_trace: &str,
    ) -> Option<&LogEntry> {
        self.content_log.iter().find(|entry| {
            entry.token_trace == token_trace
                && entry.bounce_ip == bounced_ip
                && entry.player_net_id == player_net_id
        })
    }

    /// Like [`Self::ini_bounce`], but only an entry whose address is a public one that the trace
    /// can follow on (`PassiveTrace.BounceTrace`).
    #[must_use]
    pub fn traceable_bounce(
        &self,
        bounced_ip: &str,
        player_net_id: &str,
        token_trace: &str,
    ) -> Option<&LogEntry> {
        self.ini_bounce(bounced_ip, player_net_id, token_trace)
            .filter(|entry| is_valid_ip(&entry.ip) && !is_reserved_ip(&entry.ip))
    }

    /// The log as the game saves it (`JsonConvert.SerializeObject`).
    ///
    /// # Panics
    ///
    /// Never: a log contains only strings, numbers and booleans.
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("a log is serializable")
    }
}
