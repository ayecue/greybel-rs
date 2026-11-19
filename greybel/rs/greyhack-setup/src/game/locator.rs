use std::path::Path;

use super::{GameLayout, SteamLibraryCatalog};
use crate::error::{SetupError, SetupResult};
use crate::platform::Platform;

/// Finds the installed game: in `GREY_HACK_PATH`, else in the Steam libraries.
pub struct GameLocator {
    platform: Platform,
}

impl GameLocator {
    /// The environment variable naming the game's folder.
    pub const FOLDER_VARIABLE: &'static str = "GREY_HACK_PATH";

    #[must_use]
    pub fn new(platform: Platform) -> Self {
        Self { platform }
    }

    /// The game `GREY_HACK_PATH` names, else the one in the Steam libraries; `None` when neither has it.
    pub fn locate(&self) -> SetupResult<Option<GameLayout>> {
        let folder = std::env::var_os(Self::FOLDER_VARIABLE)
            .filter(|folder| !folder.is_empty())
            .map(Into::into)
            .or_else(|| SteamLibraryCatalog::of_platform(self.platform).find_game());

        folder.map(GameLayout::from_folder).transpose()
    }

    /// The game in `folder` when given, else the located one.
    pub fn require(&self, folder: Option<&Path>) -> SetupResult<GameLayout> {
        match folder {
            Some(folder) => GameLayout::from_folder(folder),
            None => self
                .locate()?
                .ok_or(SetupError::GameMissing(Self::FOLDER_VARIABLE)),
        }
    }
}
