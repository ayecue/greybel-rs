use greybel_documents::{DocumentError, Uri};

#[derive(Debug, thiserror::Error)]
pub enum TranspileError {
    #[error(transparent)]
    Load(#[from] DocumentError),
    #[error("Operators in binary expression are not supported")]
    UnsupportedOperator {
        file: String,
        operator: &'static str,
    },
    #[error("Circular dependency from {} to {} detected.", from.label(), to.label())]
    CircularDependency { from: Uri, to: Uri },
}

pub type Result<T, E = TranspileError> = std::result::Result<T, E>;
