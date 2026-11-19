mod locator;
mod steam;

use std::path::{Path, PathBuf};

pub use self::locator::GameLocator;
pub use self::steam::SteamLibraryCatalog;
use crate::error::{SetupError, SetupResult};

/// Paths to a Grey Hack installation's executable, data folder, and BepInEx folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameLayout {
    folder: PathBuf,
    executable: PathBuf,
    data_folder: PathBuf,
}

impl GameLayout {
    /// The game in `folder`: `Grey Hack.app` on macOS, `Grey Hack.exe` on Windows, a `*.x86_64` program on Linux.
    pub fn from_folder(folder: impl Into<PathBuf>) -> SetupResult<Self> {
        let folder = folder.into();
        let app = folder.join("Grey Hack.app");

        if app.is_dir() {
            let data_folder = app.join("Contents/Resources/Data");

            return Ok(Self {
                executable: app,
                data_folder,
                folder,
            });
        }

        let exe = folder.join("Grey Hack.exe");

        if exe.is_file() {
            let data_folder = folder.join("Grey Hack_Data");

            return Ok(Self {
                executable: exe,
                data_folder,
                folder,
            });
        }

        let binary = std::fs::read_dir(&folder).ok().and_then(|entries| {
            let mut binaries: Vec<PathBuf> = entries
                .filter_map(|entry| entry.ok().map(|entry| entry.path()))
                .filter(|path| path.is_file() && path.extension().is_some_and(|e| e == "x86_64"))
                .collect();

            binaries.sort();

            binaries.into_iter().next()
        });

        match binary {
            Some(binary) => {
                let stem = binary
                    .file_stem()
                    .map(|stem| stem.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let data_folder = folder.join(format!("{stem}_Data"));

                Ok(Self {
                    executable: binary,
                    data_folder,
                    folder,
                })
            }
            None => Err(SetupError::GameMissingInFolder(folder)),
        }
    }

    #[must_use]
    pub fn folder(&self) -> &Path {
        &self.folder
    }

    #[must_use]
    pub fn executable(&self) -> &Path {
        &self.executable
    }

    #[must_use]
    pub fn data_folder(&self) -> &Path {
        &self.data_folder
    }

    #[must_use]
    pub fn managed_folder(&self) -> PathBuf {
        self.data_folder.join("Managed")
    }

    #[must_use]
    pub fn bepinex_folder(&self) -> PathBuf {
        self.folder.join("BepInEx")
    }

    #[must_use]
    pub fn plugins_folder(&self) -> PathBuf {
        self.bepinex_folder().join("plugins")
    }

    #[must_use]
    pub fn config_folder(&self) -> PathBuf {
        self.bepinex_folder().join("config")
    }
}
