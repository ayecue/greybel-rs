use mono_random::Random;

use crate::builder::build_mission;
use crate::config::{Archetype, ProcMissionsConfig, Source};
use crate::model::Mission;

pub fn build(seed: i32, source: Source) -> (Mission, Archetype) {
    build_mission(
        ProcMissionsConfig::game(),
        &mut Random::new(seed),
        10,
        source,
    )
    .expect("a mission")
}
