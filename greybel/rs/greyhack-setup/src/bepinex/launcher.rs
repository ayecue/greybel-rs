use std::path::PathBuf;

use super::BepInExMajor;
use crate::error::{SetupError, SetupResult};
use crate::game::GameLayout;

/// BepInEx's `run_bepinex.sh`, which starts the game with BepInEx on macOS and Linux.
pub struct BepInExLauncher<'a> {
    game: &'a GameLayout,
}

impl<'a> BepInExLauncher<'a> {
    pub const FILE_NAME: &'static str = "run_bepinex.sh";
    const PREFERENCE: &'static str = "archpreference=\"";

    #[must_use]
    pub fn new(game: &'a GameLayout) -> Self {
        Self { game }
    }

    #[must_use]
    pub fn path(&self) -> PathBuf {
        self.game.folder().join(Self::FILE_NAME)
    }

    /// The architectures the game must run as for `major`: BepInEx 6 for macOS is Intel only, so Apple Silicon
    /// runs the game under Rosetta.
    #[must_use]
    pub fn architectures_for(major: BepInExMajor) -> Option<&'static str> {
        (major == BepInExMajor::Beta).then_some("x86_64")
    }

    /// The value of the script's `archpreference` line.
    #[must_use]
    pub fn architectures(&self) -> Option<String> {
        let script = std::fs::read_to_string(self.path()).ok()?;

        script.lines().find_map(|line| {
            let value = line.strip_prefix(Self::PREFERENCE)?;

            value.split_once('"').map(|(value, _)| value.to_owned())
        })
    }

    /// Sets the script's first `archpreference` line to `architectures`.
    pub fn prefer_architectures(&self, architectures: &'static str) -> SetupResult<()> {
        let path = self.path();
        let script = std::fs::read_to_string(&path).map_err(SetupError::io(&path))?;
        let mut has_replacement = false;
        let lines: Vec<String> = script
            .split_inclusive('\n')
            .map(|line| {
                let remainder = line
                    .strip_prefix(Self::PREFERENCE)
                    .and_then(|value| value.split_once('"'))
                    .map(|(_, remainder)| remainder);

                match remainder {
                    Some(remainder) if !has_replacement => {
                        has_replacement = true;

                        format!("{}{architectures}\"{remainder}", Self::PREFERENCE)
                    }
                    _ => line.to_owned(),
                }
            })
            .collect();

        if !has_replacement {
            return Err(SetupError::LauncherPreferenceMissing {
                architectures,
                path,
            });
        }

        std::fs::write(&path, lines.concat()).map_err(SetupError::io(&path))
    }

    /// The Steam launch option that starts the game through the script, on macOS and Linux:
    /// `"<game>/run_bepinex.sh" %command%`.
    #[must_use]
    pub fn steam_launch_option(&self) -> String {
        format!("\"{}\" %command%", self.path().display())
    }
}
