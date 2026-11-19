use std::path::PathBuf;

/// What stops a setup step. Each message names the file, address or value it concerns.
#[derive(Debug, thiserror::Error)]
pub enum SetupError {
    #[error("This system isn't supported. Grey Hack runs on Windows, macOS and Linux.")]
    UnknownPlatform,
    #[error("Grey Hack wasn't found in {}.", .0.display())]
    GameMissingInFolder(PathBuf),
    #[error(
        "Grey Hack wasn't found in your Steam libraries. Pass its folder with --game or set {0}."
    )]
    GameMissing(&'static str),
    #[error("Couldn't download {url} (HTTP {status}).")]
    DownloadStatus { url: String, status: u16 },
    #[error("Couldn't download {url}: {message}")]
    Download { url: String, message: String },
    #[error(
        "The file from {url} failed its checksum check (expected SHA-256 {sha256}, got {actual})."
    )]
    ChecksumMismatch {
        url: String,
        sha256: String,
        actual: String,
    },
    #[error("BepInEx isn't installed. Install it first.")]
    BepInExMissing,
    #[error("{0} isn't a valid port. Use a number from 1 to 65535.")]
    PortInvalid(u32),
    #[error("{} isn't a BepInEx zip file (it has no BepInEx/core folder).", .0.display())]
    ArchiveNotBepInEx(PathBuf),
    #[error("{}: {message}", path.display())]
    Archive { path: PathBuf, message: String },
    #[error("BepInEx wasn't found after installing {}.", .0.display())]
    BepInExAbsentAfterInstall(PathBuf),
    #[error("Couldn't make the game run as {architectures}: {} has no archpreference line.", path.display())]
    LauncherPreferenceMissing {
        architectures: &'static str,
        path: PathBuf,
    },
    #[error("{} is the wrong plugin file. BepInEx {major} needs {assembly_name}.dll.", path.display())]
    HookFileWrong {
        path: PathBuf,
        major: u8,
        assembly_name: &'static str,
    },
    #[error("No release {tag} found in {repository} (HTTP {status}).")]
    ReleaseTagMissing {
        tag: String,
        repository: String,
        status: u16,
    },
    #[error("No release found in {repository} (HTTP {status}).")]
    ReleaseMissing { repository: String, status: u16 },
    #[error("Couldn't read the releases of {repository}: {message}")]
    ReleasesUnreadable { repository: String, message: String },
    #[error("Release {tag} doesn't include {name}.")]
    ReleaseAssetMissing { tag: String, name: String },
    #[error("Release {tag} has no checksum for {name}, so it can't be verified.")]
    ReleaseChecksumMissing { tag: String, name: String },
    /// A download without the user's consent.
    #[error("{0}")]
    Refusal(String),
    #[error("{}: {source}", path.display())]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl SetupError {
    pub(crate) fn io(path: impl Into<PathBuf>) -> impl FnOnce(std::io::Error) -> Self {
        let path = path.into();

        move |source| Self::Io { path, source }
    }
}

pub type SetupResult<T> = Result<T, SetupError>;
