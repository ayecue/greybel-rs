//! Code navigation tools backed by the type analyzer: document symbols, symbol search, references, descriptions,
//! the call hierarchy and rename.

mod index;
mod project;
mod target;
mod tools;

use greybel_documents::DocumentError;
use greybel_project::ProjectError;
use project::CodeProject;
use target::SymbolTarget;

use super::source::{SourceError, into_tool_contents};

#[derive(Debug, thiserror::Error)]
pub enum CodeError {
    #[error("{0}")]
    Source(#[from] SourceError),
    #[error("{0}")]
    Project(#[from] ProjectError),
    #[error("{0}")]
    Document(#[from] DocumentError),
    #[error("Provide either symbol, or path with line and column.")]
    Target,
    #[error("There is no symbol named \"{0}\".")]
    UnknownSymbol(String),
    #[error("\"{name}\" matches several symbols; use the full name or a position:\n{candidates}")]
    AmbiguousSymbol { name: String, candidates: String },
    #[error("{0} is not a GreyScript file of the project.")]
    UnknownFile(String),
    #[error("\"{0}\" is not a valid name.")]
    InvalidName(String),
    #[error("Nothing at that position can be renamed.")]
    NotRenamable,
    #[error("{path} can't be written: {error}")]
    Write { path: String, error: std::io::Error },
}

into_tool_contents!(CodeError);
