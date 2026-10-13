use greyhack_world::WorldError;

use crate::protocol::ScriptFailure;

/// Failure that prevents an operation from returning a program result.
#[derive(Debug, thiserror::Error)]
pub enum OperationError {
    /// Script failure raised with game-compatible behavior.
    #[error("{0:?}")]
    Script(ScriptFailure),
    #[error(transparent)]
    World(#[from] WorldError),
    /// Object ID is absent from the program's connection or has the wrong class.
    #[error("unknown object")]
    UnknownObject,
}

impl From<ScriptFailure> for OperationError {
    fn from(failure: ScriptFailure) -> Self {
        Self::Script(failure)
    }
}

pub type Result<T, E = OperationError> = std::result::Result<T, E>;
