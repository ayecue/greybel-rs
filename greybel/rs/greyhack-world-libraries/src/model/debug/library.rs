use greyhack_behaviour::{auto_patch_time, is_patch_applied};

use crate::ComputerId;

/// Generated debug challenge (`Challenge`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugChallenge {
    pub description: String,
    pub additional_descriptions: String,
    pub code: String,
    pub correct_code: String,
    pub error_lines: Vec<i64>,
}

/// Persistent patch state for one library file. The challenge derives from the library and clock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugLibraryState {
    pub computer: ComputerId,
    pub node: crate::NodeId,
    /// Library file path.
    pub path: String,
    pub seed: i64,
    pub auto_patch_at: Option<u64>,
    pub num_patches: u32,
    /// Patch seed, if patched (`Library.idPatch`).
    pub patch_seed: Option<i64>,
}

impl DebugLibraryState {
    /// Current challenge regenerated from the library seed
    /// (`ChallengeGenerator.GenerateChallenge`).
    #[must_use]
    pub fn challenge(&self) -> DebugChallenge {
        crate::generation::challenge::challenge(self.seed as i32)
    }

    /// Returns whether the current challenge is manually or automatically patched at world second
    /// `now` (`Library.IsPatchApplied`).
    #[must_use]
    pub fn is_patched(&self, now: u64) -> bool {
        is_patch_applied(self.seed, self.patch_seed, self.auto_patch_at, now)
    }

    /// Applies the current challenge patch (`Library.AddPatch`).
    pub fn add_patch(&mut self, now: u64) {
        self.patch_seed = Some(self.seed);
        self.auto_patch_at = Some(auto_patch_time(now, self.num_patches));
        self.num_patches += 1;
    }

    /// Resets automatic patch scheduling after a payload without clearing the manual patch
    /// (`Library.ResetNumPatches`).
    pub fn reset_num_patches(&mut self) {
        self.num_patches = 0;
        self.auto_patch_at = None;
    }
}
