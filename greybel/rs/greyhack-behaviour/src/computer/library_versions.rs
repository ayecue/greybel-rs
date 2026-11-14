//! Library releases over time: when a library version is released, how its memory zones are renewed, and how many
//! versions the world stores (`VersionsControl`).

use crate::world_seconds;

/// Real milliseconds between two checks for library versions to release (`VersionsControl.TIME_LOOP`).
pub const LIBRARY_CHECK_MILLIS: u64 = 900_000;
/// Uses of a visible weakness after which the next check releases a version patching it
/// (`VersionsControl.MAX_USED_EXPLOIT`).
pub const MAX_WEAKNESS_USES: u32 = 35;
/// Times the weaknesses of a memory zone are renewed before the zone is replaced (`VersionsControl.MAX_PATCHED_MEM`).
pub const MAX_ZONE_PATCHES: u32 = 3;
/// Bound of the memory zones a library gets (`VersionsControl.MAX_GEN_MEM`).
pub const MAX_GENERATED_ZONES: u32 = 5;
/// Library versions of a kind the world stores to give to new computers (`VersionsControl.MAX_SAVED_LIBS`).
pub const MAX_SAVED_LIBRARIES: usize = 100;
/// Draws from 0 to 9 that drop a replaced memory zone without a new one, and that make an extra zone
/// (`VersionsControl.CheckUpdatePrograms`).
const DROP_DRAWS: std::ops::RangeInclusive<u32> = 3..=5;
const EXTRA_DRAWS: std::ops::RangeInclusive<u32> = 6..=7;

/// World seconds between two checks for library versions to release (`VersionsControl.CheckVersions`).
#[must_use]
pub const fn library_check_seconds() -> u64 {
    world_seconds(LIBRARY_CHECK_MILLIS)
}

/// Returns the version that follows `version` (`VersionProgram.NewVersion`): each part counts up to 9.
#[must_use]
pub fn next_version(version: &str) -> String {
    let mut parts: Vec<u32> = version
        .split('.')
        .map(|part| part.parse().unwrap_or(0))
        .collect();

    parts.resize(3, 0);

    if parts[2] < 9 {
        parts[2] += 1;
    } else if parts[1] < 9 {
        parts[2] = 0;
        parts[1] += 1;
    } else {
        parts[0] += 1;
        parts[1] = 0;
        parts[2] = 0;
    }

    format!("{}.{}.{}", parts[0], parts[1], parts[2])
}

/// Returns the uses of a weakness after one more (`VersionsControl.UsedVulnerab`): it counts up to one past
/// [`MAX_WEAKNESS_USES`].
#[must_use]
pub const fn count_use(uses: u32) -> u32 {
    if uses <= MAX_WEAKNESS_USES {
        uses + 1
    } else {
        uses
    }
}

/// Returns whether a weakness used `uses` times gets patched by the next version
/// (`VersionsControl.CheckUpdatePrograms`).
#[must_use]
pub const fn is_worn_out(uses: u32) -> bool {
    uses >= MAX_WEAKNESS_USES
}

/// Returns whether a memory zone patched `patch_count` times is replaced rather than patched again
/// (`VersionsControl.CheckUpdatePrograms`).
#[must_use]
pub const fn is_zone_exhausted(patch_count: u32) -> bool {
    patch_count >= MAX_ZONE_PATCHES
}

/// Returns how many stored versions of a library kind are dropped, the oldest first, when `version_count` are stored
/// (`VersionsControl.PruneSavedLibVersions`).
#[must_use]
pub const fn saved_versions_to_prune(version_count: usize) -> usize {
    version_count.saturating_sub(MAX_SAVED_LIBRARIES)
}

/// Returns how many memory zones a library gets in place of `old_zones` exhausted ones
/// (`VersionsControl.CheckUpdatePrograms`), going through them last to first with a draw from 0 to 9 for each: a
/// 3 to 5 drops the zone for good while another is left, a 6 to 7 makes one more while no more than
/// [`MAX_GENERATED_ZONES`] were added, and every other zone is replaced by one. `zones` is how many zones the library
/// has, the replaced ones included; `draw` makes each draw.
#[must_use]
pub fn replacement_zone_count(
    old_zones: usize,
    zones: usize,
    mut draw: impl FnMut() -> u32,
) -> u32 {
    let mut zones = zones;
    let mut new_zones = 0;

    for _ in 0..old_zones {
        let roll = draw();

        if DROP_DRAWS.contains(&roll) {
            if zones > 1 {
                zones -= 1;

                continue;
            }
        } else if EXTRA_DRAWS.contains(&roll) && new_zones <= MAX_GENERATED_ZONES {
            new_zones += 1;
        }

        zones = zones.saturating_sub(1);
        new_zones += 1;
    }

    new_zones
}
