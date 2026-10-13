use std::sync::LazyLock;

use serde::Deserialize;

use super::Archetype;
use super::archetype::ArchetypeConfig;

/// `MissionConfig.ProcMissionsConfig`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct ProcMissionsConfig {
    pub min_level: i32,
    pub max_level: i32,
    pub levels_per_beat_step: i32,
    pub min_beats_count: i32,
    pub archetypes: Vec<ArchetypeConfig>,
}

impl Default for ProcMissionsConfig {
    fn default() -> Self {
        Self {
            min_level: 6,
            max_level: 8,
            levels_per_beat_step: 2,
            min_beats_count: 3,
            archetypes: Vec::new(),
        }
    }
}

impl ProcMissionsConfig {
    /// The game's templates (`MissionGen.procMissionsConfig`), with `ProcMissionsDefaults.EnsureShape` applied.
    pub(crate) fn game() -> &'static Self {
        static CONFIG: LazyLock<ProcMissionsConfig> = LazyLock::new(|| {
            let mut config: ProcMissionsConfig =
                serde_json::from_str(greyhack_world_assets::procedural_missions())
                    .expect("the bundled procedural missions are valid");

            config.ensure_shape();

            config
        });

        &CONFIG
    }

    /// `ProcMissionsDefaults.EnsureShape`: every archetype exists.
    pub(crate) fn ensure_shape(&mut self) {
        for archetype in Archetype::ALL {
            let index = match self
                .archetypes
                .iter()
                .position(|config| config.archetype == *archetype)
            {
                Some(index) => index,
                None => {
                    self.archetypes.push(ArchetypeConfig {
                        archetype: *archetype,
                        ..ArchetypeConfig::default()
                    });

                    self.archetypes.len() - 1
                }
            };

            self.archetypes[index].ensure_shape();
        }
    }

    /// `ProcMissionsConfig.GetProceduralLevel`: draws a level between the limits.
    pub(crate) fn procedural_level(&self, random: &mut mono_random::Random) -> i32 {
        let low = self.min_level.min(self.max_level);
        let high = self.min_level.max(self.max_level);

        low + random.next_below((high - low + 1).max(1) as usize) as i32
    }

    /// `ProcMissionsConfig.GetHeartsCount`: the number of heart beats of a mission of `level`.
    pub(crate) fn hearts_count(&self, level: i32) -> i32 {
        let steps = (level - self.min_level).max(0);

        self.min_beats_count + steps / self.levels_per_beat_step
    }
}
