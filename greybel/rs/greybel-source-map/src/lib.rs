//! Associates generated program lines with source files and lines for error reporting. All line numbers are
//! 1-based; unmapped generated lines have no source location.

mod location;
mod mapping;
mod source_map;
mod splice;
#[cfg(feature = "serde")]
mod storage;

pub use location::Location;
pub use mapping::Mapping;
pub use source_map::SourceMap;
