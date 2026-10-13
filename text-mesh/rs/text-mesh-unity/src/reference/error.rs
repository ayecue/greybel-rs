use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum ReferenceError {
    #[error("{0} is not set: run inside the repository, whose .cargo/config.toml sets it")]
    Unset(&'static str),
    #[error("the Unity editor of the reference project can't be found: {0}")]
    MissingEditor(String),
    #[error("{path}: {source}")]
    Io { path: PathBuf, source: io::Error },
    #[error("{command} failed: {detail}")]
    Command { command: String, detail: String },
    #[error("the capture is not what the project writes: {0}")]
    Capture(#[from] serde_json::Error),
}

pub(crate) trait PathContext<T> {
    fn at(self, path: &Path) -> Result<T, ReferenceError>;
}

impl<T> PathContext<T> for io::Result<T> {
    fn at(self, path: &Path) -> Result<T, ReferenceError> {
        self.map_err(|source| ReferenceError::Io {
            path: path.to_path_buf(),
            source,
        })
    }
}
