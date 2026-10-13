use greybel_core::ParseError;
use vscode_uri::Uri;

#[derive(Debug, thiserror::Error)]
pub enum DocumentError {
    #[error("{error}")]
    Parse { target: Uri, error: ParseError },
    #[error("Target {} does not exist...", target.label())]
    MissingTarget { target: Uri },
    #[error("Dependency {} does not exist...", target.label())]
    MissingDependency { target: Uri, origin: Uri },
    #[error("Injection {} does not exist...", target.label())]
    MissingInjection { target: Uri, origin: Uri },
    #[error("Cannot resolve {target}: {source}")]
    Resolve {
        target: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Cannot read {}: {source}", target.label())]
    Read {
        target: Uri,
        #[source]
        source: std::io::Error,
    },
}

pub type Result<T, E = DocumentError> = std::result::Result<T, E>;
