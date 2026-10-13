//! Bakes setups and records TMP layouts, meshes, and fonts by running the Unity reference project in batch mode.

mod asset_record;
mod reference;

pub use asset_record::{AssetRecord, RecordedRect, fingerprints_of_records, read_fingerprints};
pub use reference::{Capture, Reference, ReferenceError};
