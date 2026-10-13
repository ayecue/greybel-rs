#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UriError {
    #[error(
        "Scheme is missing: {{scheme: \"\", authority: \"{authority}\", path: \"{path}\", query: \"{query}\", fragment: \"{fragment}\"}}"
    )]
    MissingScheme {
        authority: String,
        path: String,
        query: String,
        fragment: String,
    },
    #[error("Scheme contains illegal characters.")]
    IllegalScheme,
    #[error(
        "If a URI contains an authority component, then the path component must either be empty or begin with a slash (\"/\") character"
    )]
    RelativePath,
    #[error(
        "If a URI does not contain an authority component, then the path cannot begin with two slash characters (\"//\")"
    )]
    DoubleSlashPath,
}

pub type Result<T, E = UriError> = std::result::Result<T, E>;
