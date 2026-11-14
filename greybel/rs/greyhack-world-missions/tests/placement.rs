use greyhack_behaviour::{FirewallRule, MissionKind, NetworkKind};
use greyhack_world_missions::{
    Assignment, ItemKind, MissionRequest, assign_computers, request_for,
};

use crate::support::{group, placement, request, requests, seed};

#[test]
fn should_find_a_part_for_the_networks_of_the_group_the_mission_covers() {
    for kind in [MissionKind::Secret, MissionKind::Grey, MissionKind::ZeroDay] {
        let group = group(kind, 0);
        let requests = requests(kind, 0, true);

        assert!(!requests.is_empty() && requests.len() <= group.len());

        for (at, request) in requests.iter().enumerate() {
            assert_eq!(request.index, at);
            assert_eq!(request.group, group);
            assert_eq!(request.kind(), kind);
        }

        assert!(request_for(&seed(), &group[0], false).is_some() || kind == MissionKind::ZeroDay);
    }
}

#[test]
fn should_make_a_chain_of_the_later_networks_and_of_procedural_and_hidden_missions() {
    let hidden = requests(MissionKind::Secret, 3, true);
    let grey = requests(MissionKind::Grey, 0, true);
    let zero_day = requests(MissionKind::ZeroDay, 0, true);
    let procedural = requests(MissionKind::Procedural, 0, true);

    assert!(hidden.iter().all(MissionRequest::is_chain));
    assert!(!grey[0].is_chain());
    assert!(grey[1..].iter().all(MissionRequest::is_chain));
    assert!(!zero_day[0].is_chain());
    assert!(
        zero_day[1..]
            .iter()
            .all(MissionRequest::needs_reachable_computers)
    );
    assert!(!zero_day[0].needs_reachable_computers());
    assert!(procedural[0].is_chain());
    assert!(!hidden.iter().any(MissionRequest::needs_reachable_computers));
}

#[test]
fn should_let_only_the_first_network_pick_with_its_own_generator() {
    let requests = requests(MissionKind::Grey, 2, true);

    assert!(requests[0].picks_with_network_generator());
    assert!(
        requests[1..]
            .iter()
            .all(|request| !request.picks_with_network_generator())
    );
}

#[test]
fn should_set_the_kind_of_a_later_network_from_its_first_computer() {
    let grey = requests(MissionKind::Grey, 2, true);

    assert_eq!(grey[0].network_kind(), None);
    assert_eq!(grey[1].network_kind(), Some(NetworkKind::Neurobox));

    let hidden: Vec<Option<NetworkKind>> = (0..10)
        .flat_map(|index| requests(MissionKind::Secret, index, true))
        .map(|request| request.network_kind())
        .collect();

    assert!(
        hidden
            .iter()
            .flatten()
            .all(|kind| *kind == NetworkKind::Residential)
    );
}

#[test]
fn should_pick_each_computer_once_and_remember_the_npcs_it_placed() {
    for index in 0..10 {
        for request in requests(MissionKind::Secret, index, true) {
            let count = request.part.computers.len();
            let placement = placement(request);
            let mut addresses: Vec<&str> = placement
                .assignments
                .iter()
                .map(|assignment| assignment.local_ip.as_str())
                .collect();

            assert_eq!(addresses.len(), count);

            addresses.sort_unstable();
            addresses.dedup();

            assert_eq!(addresses.len(), count);

            let mut seen = Vec::new();

            for Assignment {
                computer,
                replacement,
                npc_in_lan,
                ..
            } in &placement.assignments
            {
                let npc = placement.request.part.computers[*computer].clone_npc_pc_idx;

                assert_eq!(replacement.is_some(), npc >= 0);
                assert_eq!(*npc_in_lan, npc >= 0 && seen.contains(&npc));

                if npc >= 0 {
                    seen.push(npc);
                }
            }
        }
    }
}

#[test]
fn should_pick_the_same_computers_from_the_seed_of_the_mission() {
    let request = request(MissionKind::Secret, 3, 1);
    let available: Vec<String> = (2..30).map(|host| format!("10.0.0.{host}")).collect();

    assert_eq!(
        assign_computers(&request, available.clone(), |_| 0),
        assign_computers(&request, available, |_| 5)
    );
}

#[test]
fn should_add_the_firewall_rules_of_a_chain_after_each_computer() {
    let mut found = false;

    for index in 0..10 {
        for request in requests(MissionKind::Secret, index, true) {
            let items: Vec<usize> = request
                .part
                .computers
                .iter()
                .map(|computer| {
                    computer
                        .items
                        .iter()
                        .filter(|item| matches!(item.kind, ItemKind::Firewall(_)))
                        .count()
                })
                .collect();
            let mut expected = 0;
            let mut pending = 0;

            for count in &items {
                pending += count;
                expected += pending;
            }

            let rules: Vec<FirewallRule> = placement(request).firewall_rules();

            assert_eq!(rules.len(), expected);
            found |= !rules.is_empty();
        }
    }

    assert!(found);
}

#[test]
fn should_give_no_firewall_rules_to_a_network_that_is_not_a_chain() {
    let request = request(MissionKind::Grey, 0, 0);

    assert!(placement(request).firewall_rules().is_empty());
}

#[test]
fn should_turn_firewall_items_into_rules_with_the_addresses_of_the_group() {
    let mut rules = Vec::new();

    for index in 0..10 {
        for request in requests(MissionKind::Secret, index, true) {
            let group = request.group.clone();
            let placement = placement(request);

            for rule in placement.firewall_rules() {
                for address in rule.source.iter().chain(&rule.destination) {
                    assert!(group.contains(address));
                }

                rules.push(rule);
            }
        }
    }

    assert!(rules.iter().any(|rule| rule.port == Some(80) && rule.allow));
    assert!(
        rules
            .iter()
            .any(|rule| rule.port == Some(22) && !rule.allow)
    );
    assert!(rules.iter().any(|rule| rule.source.is_some()));
}
