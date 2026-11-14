use greyhack_behaviour::{MissionKind, mission_for_group};
use greyhack_world_missions::{
    Source, group_mission, mission_json, mission_seed, procedural_source,
};

use crate::support::{KINDS, group, seed};

#[test]
fn should_give_every_group_of_every_kind_its_mission() {
    for kind in KINDS {
        for index in 0..kind.count() {
            let group = group(kind, index);
            let mission = group_mission(&seed(), &group, kind, index, true)
                .unwrap_or_else(|| panic!("mission {index} of {}", kind.name()));

            assert_eq!(mission_for_group(&seed(), &group), Some((kind, index)));
            assert_eq!(mission.kind, kind);
            assert_eq!(mission.seed, mission_seed(&seed(), &group));
            assert!(!mission.parts.is_empty());
            assert!(mission.parts.iter().all(|part| part.kind == kind));
            assert!(mission.parts.len() <= group.len());
            assert!(
                mission
                    .parts
                    .iter()
                    .all(|part| part.reward_tier == -1 && !part.computers.is_empty())
            );
        }
    }
}

#[test]
fn should_give_the_same_mission_for_the_same_group() {
    for kind in KINDS {
        let group = group(kind, 0);

        assert_eq!(
            group_mission(&seed(), &group, kind, 0, true),
            group_mission(&seed(), &group, kind, 0, true)
        );
    }
}

#[test]
fn should_split_an_authored_mission_by_the_network_of_its_computers() {
    let group = group(MissionKind::Secret, 3);
    let mission = group_mission(&seed(), &group, MissionKind::Secret, 3, false).unwrap();

    assert_eq!(mission.parts.len(), 3);

    for (network, part) in mission.parts.iter().enumerate() {
        assert!(
            part.computers
                .iter()
                .all(|computer| computer.computer_index_net == network as i32)
        );
    }
}

#[test]
fn should_only_make_a_zero_day_mission_while_the_rumor_is_open() {
    let group = group(MissionKind::ZeroDay, 0);

    assert!(group.len() >= 4);
    assert!(group_mission(&seed(), &group, MissionKind::ZeroDay, 0, false).is_none());
    assert!(group_mission(&seed(), &group, MissionKind::ZeroDay, 0, true).is_some());
    assert!(group_mission(&seed(), &group[..3], MissionKind::ZeroDay, 0, true).is_none());
}

#[test]
fn should_make_no_mission_for_a_group_without_one() {
    let group = group(MissionKind::Grey, 0);

    assert!(group_mission(&seed(), &group, MissionKind::NoMission, 0, true).is_none());
    assert!(group_mission(&seed(), &group, MissionKind::Grey, 3, true).is_none());
}

#[test]
fn should_draw_procedural_and_authored_missions_from_the_seed_of_the_group() {
    let group = group(MissionKind::Black, 1);

    assert_ne!(
        group_mission(&seed(), &group, MissionKind::Black, 1, true),
        group_mission(&seed(), &group, MissionKind::Black, 2, true)
    );
    assert_eq!(
        procedural_source(MissionKind::PoliceProcedural),
        Source::Police
    );
    assert_eq!(procedural_source(MissionKind::Procedural), Source::Hackshop);
}

#[test]
fn should_dump_the_mission_of_every_kind() {
    for kind in KINDS {
        let group = group(kind, 0);
        let json = mission_json(&seed(), &group, kind, 0).expect("a mission");

        assert_eq!(json["missionType"], kind.name());
        assert!(
            json["parts"]
                .as_array()
                .is_some_and(|parts| !parts.is_empty())
        );
    }
}
