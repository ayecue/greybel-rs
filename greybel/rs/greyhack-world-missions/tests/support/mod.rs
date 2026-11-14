use greyhack_behaviour::{MissionKind, WorldSeed, random_group_for_mission};
use greyhack_world_missions::{MissionPlacement, MissionRequest, assign_computers, request_for};

pub fn seed() -> WorldSeed {
    WorldSeed::new(b"missions of the game".to_vec())
}

/// The group of mission `index` of `kind` in the test world.
pub fn group(kind: MissionKind, index: u32) -> Vec<String> {
    random_group_for_mission(&seed(), kind, index, 1_000).expect("a group for the mission")
}

/// The requests of the networks of the mission `index` of `kind`, in the order of the group.
pub fn requests(kind: MissionKind, index: u32, zero_day_open: bool) -> Vec<MissionRequest> {
    group(kind, index)
        .iter()
        .filter_map(|address| request_for(&seed(), address, zero_day_open))
        .collect()
}

/// The kinds of mission the game keeps in lists.
pub const LISTED: [MissionKind; 5] = [
    MissionKind::Secret,
    MissionKind::Grey,
    MissionKind::Black,
    MissionKind::White,
    MissionKind::ZeroDay,
];

/// Every kind of mission that carries one.
pub const KINDS: [MissionKind; 7] = [
    MissionKind::Secret,
    MissionKind::Grey,
    MissionKind::Black,
    MissionKind::White,
    MissionKind::ZeroDay,
    MissionKind::Procedural,
    MissionKind::PoliceProcedural,
];

pub fn request(kind: MissionKind, index: u32, network: usize) -> MissionRequest {
    requests(kind, index, true).remove(network)
}

pub fn placement(request: MissionRequest) -> MissionPlacement {
    let available: Vec<String> = (2..40).map(|host| format!("192.168.0.{host}")).collect();
    let assignments = assign_computers(&request, available, |length| length / 2);

    MissionPlacement {
        local_ips: assignments
            .iter()
            .map(|assignment| assignment.local_ip.clone())
            .collect(),
        assignments,
        websites: Vec::new(),
        request,
    }
}
