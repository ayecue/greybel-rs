//! Numeric comparison of dotted library and package versions.

use std::cmp::Ordering;

/// Initial version assigned to generated libraries and programs.
pub const FIRST_VERSION: &str = "1.0.0";

/// Compares each dotted component numerically, using 0 for non-numeric components.
#[must_use]
pub fn compare_versions(a: &str, b: &str) -> Ordering {
    let parts = |version: &str| {
        version
            .split('.')
            .map(|part| part.parse::<u32>().unwrap_or(0))
            .collect::<Vec<_>>()
    };

    parts(a).cmp(&parts(b))
}

/// Returns whether `version` has the game's version format: three dot-separated integers
/// (`VersionProgram.ConvertFormat`, `VersionProgram.IsValidVersion`). `int.TryParse` accepts surrounding white
/// space and a sign.
#[must_use]
pub fn is_valid_version(version: &str) -> bool {
    const WHITE: [char; 6] = ['\t', '\n', '\u{b}', '\u{c}', '\r', ' '];

    let parts: Vec<&str> = version.split('.').collect();

    parts.len() == 3
        && parts.iter().all(|part| {
            part.trim_start_matches(WHITE)
                .trim_end_matches(|c| WHITE.contains(&c) || c == '\0')
                .parse::<i32>()
                .is_ok()
        })
}
