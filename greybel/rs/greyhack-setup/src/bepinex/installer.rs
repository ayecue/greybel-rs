use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use zip::ZipArchive;

use super::{BepInExInstallation, BepInExLauncher, BepInExRelease};
use crate::error::{SetupError, SetupResult};
use crate::game::GameLayout;

/// Installs and removes BepInEx's loader and `core`, without changing `BepInEx/plugins` and `BepInEx/config`.
pub struct BepInExInstaller<'a> {
    game: &'a GameLayout,
}

impl<'a> BepInExInstaller<'a> {
    /// BepInEx's own folders in `BepInEx`.
    const FOLDERS: [&'static str; 5] = ["core", "patchers", "cache", "unity-libs", "interop"];
    /// The loader's files in the game's folder.
    const LOADER_FILES: [&'static str; 9] = [
        BepInExLauncher::FILE_NAME,
        "libdoorstop.dylib",
        "libdoorstop.so",
        "libdoorstop_x64.so",
        "libdoorstop_x86.so",
        "winhttp.dll",
        "doorstop_config.ini",
        ".doorstop_version",
        "changelog.txt",
    ];

    #[must_use]
    pub fn new(game: &'a GameLayout) -> Self {
        Self { game }
    }

    /// Whether the game has `release`'s major and version.
    #[must_use]
    pub fn has_release(&self, release: &BepInExRelease) -> bool {
        BepInExInstallation::detect(self.game).is_some_and(|installation| {
            installation.major == release.major && installation.version == release.version
        })
    }

    /// Replaces the game's BepInEx with the one in the zip file `archive`.
    pub fn install(&self, archive: &Path) -> SetupResult<BepInExInstallation> {
        let file = File::open(archive).map_err(SetupError::io(archive))?;
        let mut zip =
            ZipArchive::new(file).map_err(|error| Self::archive_error(archive, &error))?;

        if !zip
            .file_names()
            .any(|name| name.is_ok_and(|name| name.starts_with("BepInEx/core/")))
        {
            return Err(SetupError::ArchiveNotBepInEx(archive.to_path_buf()));
        }

        self.remove()?;
        self.extract(archive, &mut zip)?;

        let installation = BepInExInstallation::detect(self.game)
            .ok_or_else(|| SetupError::BepInExAbsentAfterInstall(archive.to_path_buf()))?;
        let launcher = BepInExLauncher::new(self.game);

        if let Some(architectures) = BepInExLauncher::architectures_for(installation.major)
            && launcher.architectures().is_some()
        {
            launcher.prefer_architectures(architectures)?;
        }

        Ok(installation)
    }

    /// Removes BepInEx's loader, its own folders and its logs.
    pub fn remove(&self) -> SetupResult<()> {
        let folder = self.game.folder();
        let bepinex = self.game.bepinex_folder();

        for file in Self::LOADER_FILES {
            Self::remove_file(&folder.join(file))?;
        }

        for name in Self::FOLDERS {
            let path = bepinex.join(name);

            if path.is_dir() {
                std::fs::remove_dir_all(&path).map_err(SetupError::io(&path))?;
            }
        }

        if let Ok(entries) = std::fs::read_dir(&bepinex) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy();

                if name.starts_with("LogOutput") && name.ends_with(".log") {
                    Self::remove_file(&entry.path())?;
                }
            }
        }

        Ok(())
    }

    fn extract(&self, archive: &Path, zip: &mut ZipArchive<File>) -> SetupResult<()> {
        let folder = self.game.folder();

        for index in 0..zip.len() {
            let mut entry = zip
                .by_index(index)
                .map_err(|error| Self::archive_error(archive, &error))?;
            let Some(relative) = entry.enclosed_name() else {
                continue;
            };
            let path = folder.join(relative);

            if entry.is_dir() {
                std::fs::create_dir_all(&path).map_err(SetupError::io(&path))?;

                continue;
            }

            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(SetupError::io(parent))?;
            }

            let mut output = File::create(&path).map_err(SetupError::io(&path))?;

            io::copy(&mut entry, &mut output).map_err(SetupError::io(&path))?;
            Self::make_executable(&path, entry.unix_mode())?;
        }

        Ok(())
    }

    /// Marks `run_bepinex.sh` and archive entries with executable permissions as executable for all users.
    #[cfg(unix)]
    fn make_executable(path: &Path, mode: Option<u32>) -> SetupResult<()> {
        use std::os::unix::fs::PermissionsExt;

        let is_launcher = path
            .file_name()
            .is_some_and(|name| name == BepInExLauncher::FILE_NAME);

        if !is_launcher && mode.is_none_or(|mode| mode & 0o111 == 0) {
            return Ok(());
        }

        let mut permissions = std::fs::metadata(path)
            .map_err(SetupError::io(path))?
            .permissions();

        permissions.set_mode(permissions.mode() | 0o111);

        std::fs::set_permissions(path, permissions).map_err(SetupError::io(path))
    }

    #[cfg(not(unix))]
    #[expect(clippy::unnecessary_wraps, reason = "matches the Unix version")]
    fn make_executable(_path: &Path, _mode: Option<u32>) -> SetupResult<()> {
        Ok(())
    }

    fn remove_file(path: &Path) -> SetupResult<()> {
        match std::fs::remove_file(path) {
            Err(error) if error.kind() != io::ErrorKind::NotFound => {
                Err(SetupError::io(path)(error))
            }
            _ => Ok(()),
        }
    }

    fn archive_error(archive: &Path, error: &zip::result::ZipError) -> SetupError {
        SetupError::Archive {
            path: PathBuf::from(archive),
            message: error.to_string(),
        }
    }
}
