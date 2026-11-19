use std::path::{Path, PathBuf};

use crate::bepinex::{BepInExInstallation, BepInExInstaller, BepInExMajor, BepInExRelease};
use crate::download::Downloader;
use crate::error::{SetupError, SetupResult};
use crate::game::GameLayout;
use crate::hook::{HookConfig, HookInstaller, HookRelease};
use crate::platform::Platform;
use crate::status::GameStatus;

/// Where BepInEx comes from: the pinned release of a major, downloaded, or a zip file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BepInExSource {
    Download(BepInExMajor),
    Archive(PathBuf),
}

/// Where the message hook comes from: a GitHub release, or an assembly file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookSource {
    Release(HookRelease),
    Assembly(PathBuf),
}

/// The setup steps for one game. Each step reports what it did through `progress`, one sentence at a time.
pub struct Setup<'a> {
    game: GameLayout,
    platform: Platform,
    downloader: &'a Downloader,
}

impl<'a> Setup<'a> {
    #[must_use]
    pub fn new(game: GameLayout, platform: Platform, downloader: &'a Downloader) -> Self {
        Self {
            game,
            platform,
            downloader,
        }
    }

    #[must_use]
    pub fn game(&self) -> &GameLayout {
        &self.game
    }

    #[must_use]
    pub fn status(&self) -> GameStatus {
        GameStatus::read(&self.game, self.platform)
    }

    /// The BepInEx `major` archive for this platform.
    #[must_use]
    pub fn bepinex_release(&self, major: BepInExMajor) -> BepInExRelease {
        BepInExRelease::of(major, self.platform)
    }

    /// The installed BepInEx's major.
    pub fn current_bepinex_major(&self) -> SetupResult<BepInExMajor> {
        BepInExInstallation::detect(&self.game)
            .map(|installation| installation.major)
            .ok_or(SetupError::BepInExMissing)
    }

    pub fn install_bepinex(
        &self,
        source: &BepInExSource,
        progress: &mut dyn FnMut(String),
    ) -> SetupResult<BepInExInstallation> {
        let archive = match source {
            BepInExSource::Archive(archive) => archive.clone(),
            BepInExSource::Download(major) => {
                let release = self.bepinex_release(*major);

                progress(format!(
                    "Downloading BepInEx {} from {}",
                    release.version, release.url
                ));

                self.downloader
                    .fetch(&release.url, "", &release.file_name, release.sha256)?
            }
        };

        progress(format!("Installing BepInEx from {}", archive.display()));

        let installation = BepInExInstaller::new(&self.game).install(&archive)?;

        progress(format!(
            "Installed {} in {}",
            installation.description(),
            self.game.folder().display()
        ));

        Ok(installation)
    }

    pub fn remove_bepinex(&self, progress: &mut dyn FnMut(String)) -> SetupResult<()> {
        BepInExInstaller::new(&self.game).remove()?;
        progress(format!(
            "Removed BepInEx from {}. Plugins and settings were not affected.",
            self.game.folder().display()
        ));

        Ok(())
    }

    /// Installs the hook for `major`, by default the installed BepInEx's.
    pub fn install_hook(
        &self,
        major: Option<BepInExMajor>,
        source: &HookSource,
        progress: &mut dyn FnMut(String),
    ) -> SetupResult<PathBuf> {
        let major = match major {
            Some(major) => major,
            None => self.current_bepinex_major()?,
        };
        let assembly = match source {
            HookSource::Assembly(assembly) => assembly.clone(),
            HookSource::Release(release) => {
                let asset = release.asset(major)?;

                progress(format!(
                    "Downloading {} ({}) from {}",
                    asset.name, release.tag, asset.url
                ));

                release.fetch(self.downloader, major)?
            }
        };
        let target = HookInstaller::new(&self.game).install(major, &assembly)?;

        progress(format!(
            "Installed {} in {}",
            Self::file_name(&target),
            self.game.plugins_folder().display()
        ));

        Ok(target)
    }

    pub fn remove_hook(&self, progress: &mut dyn FnMut(String)) -> SetupResult<()> {
        HookInstaller::new(&self.game).remove()?;
        progress(format!(
            "Removed the message hook from {}",
            self.game.plugins_folder().display()
        ));

        Ok(())
    }

    pub fn configure_port(&self, port: u32, progress: &mut dyn FnMut(String)) -> SetupResult<()> {
        let port = u16::try_from(port)
            .ok()
            .filter(|&port| port != 0)
            .ok_or(SetupError::PortInvalid(port))?;
        let config = HookConfig::new(&self.game);

        config.set_port(port)?;
        progress(format!(
            "Set the port to {port} in {}",
            config.path().display()
        ));

        Ok(())
    }

    /// Removes the `libsteam_api.dylib` [`GameStatus::duplicate_steam_api`] names.
    pub fn remove_duplicate_steam_api(&self, progress: &mut dyn FnMut(String)) -> SetupResult<()> {
        if let Some(path) = self.status().duplicate_steam_api {
            std::fs::remove_file(&path).map_err(SetupError::io(&path))?;
            progress(format!("Removed {}", path.display()));
        }

        Ok(())
    }

    fn file_name(path: &Path) -> String {
        path.file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default()
    }
}
