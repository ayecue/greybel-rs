//! Shared helpers for program integration tests.

pub mod object;
pub mod program;

pub(crate) use object::{HandleId, Other};
pub(crate) use program::{id, push, terminal, tree};
