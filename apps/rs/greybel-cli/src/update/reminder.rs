use std::io::IsTerminal;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use greyhack_setup::{Downloader, Platform};
use serde::{Deserialize, Serialize};

use super::UpdateCheck;
use super::feed::ReleaseFeed;

/// Displays a daily update notice in terminals. Reads the cached release version and refreshes it on a background
/// thread when the previous check is older than one day.
pub struct UpdateReminder {
    file: PathBuf,
}

/// The reminder's file: when the newest version was last checked, and that version.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CheckRecord {
    last_check: u64,
    version: Option<String>,
}

impl UpdateReminder {
    /// The environment variable that turns the reminder off.
    pub const DISABLE_VARIABLE: &'static str = "GREYBEL_NO_UPDATE_CHECK";
    const INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

    /// The reminder, unless stderr isn't a terminal or `GREYBEL_NO_UPDATE_CHECK` is set.
    pub fn new() -> Option<Self> {
        if std::env::var_os(Self::DISABLE_VARIABLE).is_some_and(|value| !value.is_empty())
            || !std::io::stderr().is_terminal()
        {
            return None;
        }

        let folder = Platform::current().ok()?.cache_folder()?;

        Some(Self {
            file: folder.join("greybel").join("update-check.json"),
        })
    }

    /// Starts a check when the last one is a day old.
    pub fn refresh(&self) {
        let record = self.read();
        let now = Self::now();

        if now.saturating_sub(record.last_check) < Self::INTERVAL.as_secs() {
            return;
        }

        let file = self.file.clone();

        std::thread::spawn(move || {
            let downloader =
                Downloader::with_user_agent(std::env::temp_dir(), UpdateCheck::USER_AGENT);
            let version = ReleaseFeed::new()
                .latest(&downloader)
                .ok()
                .map(|release| release.version);
            let record = CheckRecord {
                last_check: now,
                version,
            };

            if let Some(folder) = file.parent()
                && std::fs::create_dir_all(folder).is_ok()
                && let Ok(text) = serde_json::to_string(&record)
            {
                let _ = std::fs::write(&file, text);
            }
        });
    }

    /// One line on the newer version the last check found, if any.
    pub fn notice(&self) -> Option<String> {
        let version = self.read().version?;

        (super::feed::ReleaseVersion::compare(&version, UpdateCheck::VERSION)
            == std::cmp::Ordering::Greater)
            .then(|| {
                format!(
                    "greybel {version} is available (this is {}): run `greybel update`.",
                    UpdateCheck::VERSION
                )
            })
    }

    fn read(&self) -> CheckRecord {
        std::fs::read_to_string(&self.file)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_secs())
    }
}
