use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use greyhack_setup::Downloader;

use super::feed::{LatestRelease, ReleaseAsset};
use crate::report::Failure;

/// Replaces the running `greybel` with the program in a release's archive for this build's Rust target.
pub struct SelfUpdate {
    folder: PathBuf,
}

impl SelfUpdate {
    const PROGRAM: &'static str = if cfg!(windows) {
        "greybel.exe"
    } else {
        "greybel"
    };
    /// The Rust target this program is built for, such as `aarch64-apple-darwin`.
    pub const TARGET: &'static str = env!("GREYBEL_TARGET");

    /// Creates a self-update whose temporary folder is removed on drop.
    pub fn new() -> Self {
        Self {
            folder: std::env::temp_dir().join(format!("greybel-update-{}", std::process::id())),
        }
    }

    /// The archive of `release` for [`Self::TARGET`].
    pub fn asset(release: &LatestRelease) -> Result<&ReleaseAsset, Failure> {
        release.assets.get(Self::TARGET).ok_or_else(|| {
            Failure::new(
                "Update error",
                format!(
                    "greybel {} has no build for {}. Download a release from https://github.com/ayecue/greybel-rs/releases.",
                    release.version,
                    Self::TARGET
                ),
            )
        })
    }

    /// Downloads `asset`, checks its SHA-256 and replaces the running program with the one in it.
    pub fn install(&self, asset: &ReleaseAsset, user_agent: &str) -> Result<(), Failure> {
        let failure = |message: String| Failure::new("Update error", message);
        let file_name = asset.url.rsplit('/').next().unwrap_or("greybel-archive");
        let archive = Downloader::with_user_agent(self.folder.clone(), user_agent)
            .fetch(&asset.url, "", file_name, &asset.sha256)
            .map_err(|error| failure(error.to_string()))?;
        let program = self.folder.join(Self::PROGRAM);

        if file_name.ends_with(".zip") {
            Self::extract_zip(&archive, &program)
        } else {
            Self::extract_tar_gz(&archive, &program)
        }
        .map_err(|error| failure(format!("{}: {error}", archive.display())))?;

        self_replace::self_replace(&program).map_err(|error| {
            let current = std::env::current_exe()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|_| "greybel".to_owned());

            failure(format!(
                "Couldn't replace {current}: {error}. Download {} and replace it yourself.",
                asset.url
            ))
        })
    }

    fn extract_tar_gz(archive: &Path, program: &Path) -> io::Result<()> {
        let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(File::open(archive)?));

        for entry in tar.entries()? {
            let mut entry = entry?;

            if entry
                .path()?
                .file_name()
                .is_some_and(|name| name == Self::PROGRAM)
            {
                io::copy(&mut entry, &mut File::create(program)?)?;

                return Self::make_executable(program);
            }
        }

        Err(Self::absent_program_error())
    }

    fn extract_zip(archive: &Path, program: &Path) -> io::Result<()> {
        let mut zip = zip::ZipArchive::new(File::open(archive)?).map_err(io::Error::other)?;

        for index in 0..zip.len() {
            let mut entry = zip.by_index(index).map_err(io::Error::other)?;

            if entry
                .enclosed_name()
                .is_some_and(|path| path.file_name().is_some_and(|name| name == Self::PROGRAM))
            {
                io::copy(&mut entry, &mut File::create(program)?)?;

                return Self::make_executable(program);
            }
        }

        Err(Self::absent_program_error())
    }

    fn absent_program_error() -> io::Error {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("the archive has no {}", Self::PROGRAM),
        )
    }

    #[cfg(unix)]
    fn make_executable(program: &Path) -> io::Result<()> {
        use std::os::unix::fs::PermissionsExt;

        std::fs::set_permissions(program, std::fs::Permissions::from_mode(0o755))
    }

    #[cfg(not(unix))]
    #[expect(clippy::unnecessary_wraps, reason = "matches the Unix version")]
    fn make_executable(_program: &Path) -> io::Result<()> {
        Ok(())
    }
}

impl Drop for SelfUpdate {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.folder);
    }
}
