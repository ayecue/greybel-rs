//! Which network of a mission a public address is, and where the mission puts its computers
//! (`NetworkLan.ConfigureMissions`, `NetworkLan.CreateMission`).

use greyhack_behaviour::{MissionKind, WorldSeed, ip_group, mission_for_group};
use mono_random::Random;

use super::builder::build_mission;
use super::config::{ProcMissionsConfig, Source};
use super::model::{Mission, PreGenNpc};

/// The part of a procedural mission a network has (`NetworkLan.missionParts[indexIp]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MissionRequest {
    /// Addresses of the networks of the mission (`NetworkLan.groupIPs`).
    pub group: Vec<String>,
    /// Position of the network in the group.
    pub index: usize,
    /// Seed of the mission (`MissionGen.GetMissionSeed`).
    pub seed: i32,
    pub part: Mission,
}

/// Where a computer of the mission went.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Assignment {
    pub local_ip: String,
    /// Index into the computers of the part.
    pub computer: usize,
    /// The draw that picks which account of the computer the mission NPC replaces
    /// (`NetworkLan.GetMissionNpcReplacementIndex`), when the mission puts an NPC on it.
    pub replacement: Option<f64>,
    /// The same mission NPC is already on another computer of the network (`NetworkLan.IsMissionNpcAlreadyInLan`).
    pub npc_in_lan: bool,
}

/// What a network was given by its mission.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MissionPlacement {
    pub request: MissionRequest,
    pub assignments: Vec<Assignment>,
    /// Local addresses of the mission computers in the order of the mission (`NetworkLan.GetLocalIPsIdx`).
    pub local_ips: Vec<String>,
    /// Computers that carry the text of a website, with the text (`LanPcInfo.website`).
    pub websites: Vec<(String, String)>,
}

impl MissionPlacement {
    pub(crate) fn npcs(&self) -> &[PreGenNpc] {
        &self.request.part.pre_gen_npcs
    }

    pub(crate) fn assignment_at(&self, local_ip: &str) -> Option<&Assignment> {
        self.assignments
            .iter()
            .find(|assignment| assignment.local_ip == local_ip)
    }
}

/// The procedural mission of the network at `public_ip` (`NetworkLan.ConfigureMissions`): `None` where the address
/// has no mission of that kind, or none for its position in the group.
pub(crate) fn request_for(seed: &WorldSeed, public_ip: &str) -> Option<MissionRequest> {
    let group = ip_group(seed, public_ip)?;
    let (kind, _) = mission_for_group(seed, &group)?;

    if !kind.is_procedural() {
        return None;
    }

    let source = if kind == MissionKind::PoliceProcedural {
        Source::Police
    } else {
        Source::Hackshop
    };
    let mission_seed = seed.derive_int(
        "mission-seed-v1",
        &group.iter().map(String::as_str).collect::<Vec<_>>(),
    );
    let (mission, _) = build_mission(
        ProcMissionsConfig::game(),
        &mut Random::new(mission_seed),
        group.len(),
        source,
    )
    .ok()?;
    let index = group.iter().position(|address| address == public_ip)?;
    let part = mission.split().into_iter().nth(index)?;

    Some(MissionRequest {
        group,
        index,
        seed: mission_seed,
        part,
    })
}

/// The part of the mission of network `index` of the mission of `group`, built as the game builds it for any of
/// its networks (`MissionGen.GetProceduralMission`).
pub(crate) fn mission_of_group(
    seed: &WorldSeed,
    group: &[String],
    source: Source,
) -> Option<(i32, Mission)> {
    let mission_seed = seed.derive_int(
        "mission-seed-v1",
        &group.iter().map(String::as_str).collect::<Vec<_>>(),
    );
    let (mission, _) = build_mission(
        ProcMissionsConfig::game(),
        &mut Random::new(mission_seed),
        group.len(),
        source,
    )
    .ok()?;

    Some((mission_seed, mission))
}
