//! Timestamps of generated files (`Fichero.InitializeTimestamp`).

/// Computes FNV-1a over UTF-16 code units like `Fichero.InitializeTimestamp` and `XmlGlobal.CloneTemplate`.
#[must_use]
pub fn timestamp_hash(text: &str) -> u32 {
    text.encode_utf16().fold(2_166_136_261_u32, |hash, unit| {
        (hash ^ u32::from(unit)).wrapping_mul(16_777_619)
    })
}

/// Minutes in the longest age a generated entry has beyond its parent's.
const TEMPLATE_AGE_RANGE_MINUTES: u32 = 43_200;

/// Age of an entry that is not dated, as its parent's timestamp minus a stable number of minutes
/// derived from its `name` (`Fichero.InitializeTimestamp`).
#[must_use]
pub fn template_timestamp(parent_timestamp: u64, name: &str) -> u64 {
    let age_minutes = 1 + u64::from(timestamp_hash(name) % TEMPLATE_AGE_RANGE_MINUTES);

    parent_timestamp.saturating_sub(age_minutes * 60)
}
