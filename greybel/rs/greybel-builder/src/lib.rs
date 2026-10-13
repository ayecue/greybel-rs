//! Provides GreyScript project builds, file imports, and source transformations ([`build()`], [`import()`],
//! [`transform()`]) with pluggable document providers. [`Builder`] serves these operations over duplex and obtains
//! editor files through the documents protocol.

mod build;
mod builder;
mod deploy;
mod import;
pub mod protocol;

pub use build::{build, transform};
pub use builder::Builder;
pub use import::import;
