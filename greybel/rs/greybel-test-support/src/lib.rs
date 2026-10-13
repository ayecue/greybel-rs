//! Shared fixtures and harnesses for greybel crates. Crate-specific helpers stay in `tests/support/`; these modules
//! are cross-crate and gated by same-named features.

pub mod protocols;

/// Reexports duplex-crate helpers for driving futures and creating temporary folders.
pub use duplex_test_support::{TempFolder, future};

#[cfg(feature = "documents")]
pub mod documents;

#[cfg(feature = "interpreter")]
pub mod interpreter;

#[cfg(feature = "greyhack")]
pub mod greyhack;

#[cfg(feature = "world")]
pub mod world;

#[cfg(feature = "game")]
pub mod game;

#[cfg(feature = "file-editor")]
pub mod file_editor;

#[cfg(feature = "io")]
pub mod io;

#[cfg(feature = "lsp")]
pub mod lsp;

#[cfg(feature = "terminal")]
pub mod terminal;

#[cfg(feature = "meta")]
pub mod meta;

#[cfg(feature = "textmate")]
pub mod textmate;

#[cfg(feature = "node-depth")]
pub mod node_depth;
