//! Patches and zero-day payloads of debug libraries (`Library`, `ZeroDaySystem`,
//! `DebugLibraryIntrinsics`).

use crate::{GUEST_USER, ROOT_USER, SECONDS_PER_DAY, add_months, date_parts};

/// Whether the library is patched against the challenge of `challenge_seed` at world second `now`:
/// manually patched for that seed, or inside the automatic patch window (`Library.IsPatchApplied`).
#[must_use]
pub fn is_patch_applied(
    challenge_seed: i64,
    patch_seed: Option<i64>,
    auto_patch_at: Option<u64>,
    now: u64,
) -> bool {
    patch_seed == Some(challenge_seed) || auto_patch_at.is_some_and(|until| now < until)
}

/// Returns the world second at which the automatic patch of a library patched `num_patches` times
/// before ends: the first of this month plus the capped patch-count interval
/// (`ZeroDaySystem.GetAutoPatchTime`).
#[must_use]
pub fn auto_patch_time(now: u64, num_patches: u32) -> u64 {
    let (_, _, day) = date_parts(now);
    let first = now - (u64::from(day - 1) * SECONDS_PER_DAY + now % SECONDS_PER_DAY);
    let months = ((num_patches + 1) * 2 + 1).min(7);

    add_months(first, months)
}

/// Whether the lines a script passes to `unit_testing` are the error lines of the challenge,
/// ignoring order and repeats (`ErrorChecker.AreErrorLinesCorrect`).
#[must_use]
pub fn error_lines_match(script_lines: &[i64], challenge_lines: &[i64]) -> bool {
    let normalize = |lines: &[i64]| {
        let mut lines = lines.to_vec();

        lines.sort_unstable();
        lines.dedup();

        lines
    };

    normalize(script_lines) == normalize(challenge_lines)
}

/// Returns the account a payload object runs as when its access names `root` or `guest`; any other
/// access picks a regular user of the computer (`DebugLibraryIntrinsics.payload`).
#[must_use]
pub fn fixed_payload_user(access: &str) -> Option<&str> {
    (access == ROOT_USER || access == GUEST_USER).then_some(access)
}
