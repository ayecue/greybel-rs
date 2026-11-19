use std::path::{Path, PathBuf};

use crate::platform::Platform;

/// The Steam libraries of a user, and the Grey Hack installation in them.
pub struct SteamLibraryCatalog {
    steam_folders: Vec<PathBuf>,
}

impl SteamLibraryCatalog {
    /// Grey Hack's Steam app id.
    pub const APP_ID: &'static str = "605230";

    #[must_use]
    pub fn new(steam_folders: Vec<PathBuf>) -> Self {
        Self { steam_folders }
    }

    /// Steam's usual folders on `platform`.
    #[must_use]
    pub fn of_platform(platform: Platform) -> Self {
        let home = platform.home().unwrap_or_default();
        let variable = |name: &str| std::env::var_os(name).map(PathBuf::from);
        let steam_folders = match platform {
            Platform::Windows => ["ProgramFiles(x86)", "ProgramFiles"]
                .into_iter()
                .filter_map(variable)
                .map(|programs| programs.join("Steam"))
                .collect(),
            Platform::MacOS => vec![home.join("Library/Application Support/Steam")],
            Platform::Linux => vec![
                home.join(".steam/steam"),
                home.join(".local/share/Steam"),
                home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"),
            ],
        };

        Self::new(steam_folders)
    }

    /// The library folders in the text of a `libraryfolders.vdf`: the value of each `"path"` key.
    #[must_use]
    pub fn library_paths(library_folders: &str) -> Vec<PathBuf> {
        const KEY: &str = "\"path\"";

        library_folders
            .match_indices(KEY)
            .filter_map(|(start, _)| {
                let value = library_folders[start + KEY.len()..].trim_start();
                let value = value.strip_prefix('"')?;
                let end = value.find('"')?;

                Some(PathBuf::from(value[..end].replace("\\\\", "\\")))
            })
            .collect()
    }

    /// The `Grey Hack` folder of the first library with the game's app manifest.
    #[must_use]
    pub fn find_game(&self) -> Option<PathBuf> {
        self.steam_folders
            .iter()
            .flat_map(|steam| Self::libraries(steam))
            .find_map(|library| {
                let apps = library.join("steamapps");
                let game = apps.join("common/Grey Hack");
                let manifest = apps.join(format!("appmanifest_{}.acf", Self::APP_ID));

                (manifest.is_file() && game.is_dir()).then_some(game)
            })
    }

    fn libraries(steam: &Path) -> Vec<PathBuf> {
        let mut libraries = vec![steam.to_path_buf()];

        if let Ok(text) = std::fs::read_to_string(steam.join("steamapps/libraryfolders.vdf")) {
            for library in Self::library_paths(&text) {
                if !libraries.contains(&library) {
                    libraries.push(library);
                }
            }
        }

        libraries
    }
}
