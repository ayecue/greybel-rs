use mono_random::Random;

use crate::generation::missions::builder::build_mission;
use crate::generation::missions::config::{Archetype, ProcMissionsConfig, Source};
use crate::generation::missions::model::Mission;

pub fn build(seed: i32, source: Source) -> (Mission, Archetype) {
    build_mission(
        ProcMissionsConfig::game(),
        &mut Random::new(seed),
        10,
        source,
    )
    .expect("a mission")
}
