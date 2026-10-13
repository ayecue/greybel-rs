use greyhack_behaviour::{
    MissionKind, ip_group, is_group_reserved_ip, mission_for_group, random_group_for_mission,
};

use crate::support::mission_groups::seed;

#[test]
fn should_give_every_address_of_a_group_the_same_group() {
    let seed = seed();
    let group = ip_group(&seed, "93.18.201.7").unwrap();

    assert!(!group.is_empty() && group.len() <= 10);
    assert!(group.iter().all(|address| !is_group_reserved_ip(address)));

    for address in &group {
        assert_eq!(ip_group(&seed, address).unwrap(), group);
    }
}

#[test]
fn should_reject_an_invalid_address() {
    assert_eq!(ip_group(&seed(), "not an address"), None);
    assert_eq!(mission_for_group(&seed(), &[]), None);
}

#[test]
fn should_find_a_group_of_each_mission_of_a_kind() {
    let seed = seed();

    for kind in [MissionKind::Procedural, MissionKind::PoliceProcedural] {
        for index in 0..kind.count() {
            let group = random_group_for_mission(&seed, kind, index, 7_000_000 + index).unwrap();

            assert_eq!(mission_for_group(&seed, &group), Some((kind, index)));
        }
    }
}

#[test]
fn should_refuse_a_mission_index_beyond_the_kind() {
    let seed = seed();

    assert_eq!(
        random_group_for_mission(&seed, MissionKind::Procedural, 3, 0),
        None
    );
}

#[test]
fn should_only_give_zero_day_groups_of_four_addresses() {
    let group = random_group_for_mission(&seed(), MissionKind::ZeroDay, 0, 123).unwrap();

    assert!(group.len() >= 4);
}

#[test]
fn should_search_from_the_start_group_number() {
    let seed = seed();
    let first = random_group_for_mission(&seed, MissionKind::Procedural, 0, 5).unwrap();
    let again = random_group_for_mission(&seed, MissionKind::Procedural, 0, 5).unwrap();

    assert_eq!(first, again);
}
