use greybel_documents::{DocumentError, Uri};

#[derive(Debug, thiserror::Error)]
pub enum TranspileError {
    #[error(transparent)]
    Load(#[from] DocumentError),
    #[error("The operator \"{operator}\" is not supported.")]
    UnsupportedOperator {
        file: String,
        operator: &'static str,
    },
    #[error(
        "{file}:{line}:{column}: a ternary expression, `??` or optional chain can't be built here; builds compute it \
         before its statement, which this position (the right side of and/or, a parameter default or an assert \
         message) only runs sometimes"
    )]
    UnexpandedValue {
        file: String,
        line: u32,
        column: u32,
    },
    #[error("Circular dependency from {} to {}.", from.label(), to.label())]
    CircularDependency { from: Uri, to: Uri },
}

pub type Result<T, E = TranspileError> = std::result::Result<T, E>;
