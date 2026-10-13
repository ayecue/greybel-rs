//! Generates TypeScript [`Module`]s for one duplex definition major version for the `duplex-core` TypeScript package.

#![warn(missing_docs)]
mod ast;
mod codec;
mod declaration;
mod module;
mod types;

pub use duplex_protocol::{MinorVersionTable, Role, VersionHistory};
pub use module::Module;
