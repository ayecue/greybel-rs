use greybel_documents::DocumentError;

use crate::error::{CompileError, RuntimeError};

#[derive(Debug, thiserror::Error)]
pub enum InterpreterError {
    #[error(transparent)]
    Documents(#[from] DocumentError),
    #[error(transparent)]
    Compile(#[from] CompileError),
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error("No program is running.")]
    NoProgram,
}
