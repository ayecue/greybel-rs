use std::path::PathBuf;

use crate::error::{SetupError, SetupResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Windows,
    MacOS,
    Linux,
}

impl Platform {
    /// The system this program runs on.
    pub fn current() -> SetupResult<Self> {
        if cfg!(windows) {
            Ok(Self::Windows)
        } else if cfg!(target_os = "macos") {
            Ok(Self::MacOS)
        } else if cfg!(target_os = "linux") {
            Ok(Self::Linux)
        } else {
            Err(SetupError::UnknownPlatform)
        }
    }

    /// The user's home folder.
    #[must_use]
    pub fn home(self) -> Option<PathBuf> {
        let variable = if self == Self::Windows {
            "USERPROFILE"
        } else {
            "HOME"
        };

        std::env::var_os(variable)
            .filter(|home| !home.is_empty())
            .map(PathBuf::from)
    }

    /// The user's folder for files that can be downloaded again: `%LOCALAPPDATA%` on Windows, `Library/Caches` on
    /// macOS, `$XDG_CACHE_HOME` or `~/.cache` on Linux.
    #[must_use]
    pub fn cache_folder(self) -> Option<PathBuf> {
        let variable = |name: &str| {
            std::env::var_os(name)
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
        };

        match self {
            Self::Windows => variable("LOCALAPPDATA"),
            Self::MacOS => self.home().map(|home| home.join("Library/Caches")),
            Self::Linux => {
                variable("XDG_CACHE_HOME").or_else(|| self.home().map(|home| home.join(".cache")))
            }
        }
    }
}
