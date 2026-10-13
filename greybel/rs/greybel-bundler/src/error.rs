use greybel_documents::{DocumentError, Uri};
use greybel_transpiler::TranspileError;

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("No entry files to build.")]
    NoEntries,
    #[error("An output name needs exactly one entry file, not {0}.")]
    OutputNameWithEntries(usize),
    #[error("{} and {} share no folder.", .0.label(), .1.label())]
    NoCommonFolder(Uri, Uri),
    #[error(
        "Installer scripts of at most {max_chars} characters cannot fit a file's installer code ({required_chars} characters)."
    )]
    MaxCharsTooSmall {
        max_chars: usize,
        required_chars: usize,
    },
    #[error(transparent)]
    Transpile(#[from] TranspileError),
}

impl From<DocumentError> for BuildError {
    fn from(error: DocumentError) -> Self {
        Self::Transpile(TranspileError::Load(error))
    }
}

pub type Result<T, E = BuildError> = std::result::Result<T, E>;
