//! The mission of a group of networks, whatever its kind (`MissionGen.GetMission`, `MissionGen.GetProceduralMission`,
//! `NetworkLan.ConfigureMissions`).

use greyhack_behaviour::{MissionKind, WorldSeed};
use mono_random::Random;

use crate::builder::build_mission;
use crate::config::{MissionCatalog, ProcMissionsConfig, Source};
use crate::model::Mission;

/// The mission a group of networks carries, split into the part of each network.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupMission {
    pub kind: MissionKind,
    /// Seed of the mission (`MissionGen.GetMissionSeed`).
    pub seed: i32,
    /// `MissionGen.SplitMission` of the mission.
    pub parts: Vec<Mission>,
}

/// The source of a procedural mission kind (`MissionGen.GetProceduralSource`).
pub fn procedural_source(kind: MissionKind) -> Source {
    if kind == MissionKind::PoliceProcedural {
        Source::Police
    } else {
        Source::Hackshop
    }
}

/// The mission `index` of `kind` for `group`, built from the seed of the group as `NetworkLan.ConfigureMissions` does:
/// a procedural kind by `ProcMissionBuilder`, the others from the game's authored lists. `None` where the group has no
/// mission: a group without a mission, a zero-day mission while none is generated (`zero_day_open` is
/// `ZeroDaySystem.IsRumorTime`) or in a group of fewer than four networks, or a mission the game cannot build.
pub fn group_mission(
    seed: &WorldSeed,
    group: &[String],
    kind: MissionKind,
    index: u32,
    zero_day_open: bool,
) -> Option<GroupMission> {
    if kind == MissionKind::NoMission
        || kind == MissionKind::ZeroDay && !(zero_day_open && group.len() >= 4)
    {
        return None;
    }

    let mission_seed = mission_seed(seed, group);
    let mut random = Random::new(mission_seed);
    let mission = if kind.is_procedural() {
        let (mission, _) = build_mission(
            ProcMissionsConfig::game(),
            &mut random,
            group.len(),
            procedural_source(kind),
        )
        .ok()?;

        Mission { kind, ..mission }
    } else {
        MissionCatalog::of(kind)?.instantiate(usize::try_from(index).ok()?, &mut random)?
    };

    Some(GroupMission {
        kind,
        seed: mission_seed,
        parts: mission.split(),
    })
}

/// `MissionGen.GetMissionSeed`.
pub fn mission_seed(seed: &WorldSeed, group: &[String]) -> i32 {
    seed.derive_int(
        "mission-seed-v1",
        &group.iter().map(String::as_str).collect::<Vec<_>>(),
    )
}
