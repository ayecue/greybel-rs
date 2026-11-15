use greyhack_behaviour::MissionKind;
use greyhack_world_missions::{ItemKind, MissionCatalog};
use mono_random::Random;

use crate::support::LISTED;

#[test]
fn should_hold_the_number_of_missions_the_game_counts_for_each_kind() {
    let lengths: Vec<(usize, u32)> = LISTED
        .iter()
        .map(|kind| (MissionCatalog::of(*kind).unwrap().len(), kind.count()))
        .collect();

    assert!(lengths.iter().all(|(length, count)| {
        u32::try_from(*length).expect("value fits the target integer type") == *count
    }));
}

#[test]
fn should_not_author_the_other_kinds() {
    for kind in [
        MissionKind::Procedural,
        MissionKind::PoliceProcedural,
        MissionKind::NoMission,
    ] {
        assert!(MissionCatalog::of(kind).is_none());
    }
}

#[test]
fn should_read_every_kind_of_item_the_game_authors() {
    let mut kinds = [false; 6];

    for kind in LISTED {
        let missions = MissionCatalog::of(kind).unwrap();

        for index in 0..missions.len() {
            let mission = missions.instantiate(index, &mut Random::new(1)).unwrap();

            for item in mission
                .computers
                .iter()
                .flat_map(|computer| &computer.items)
            {
                let at = match item.kind {
                    ItemKind::ChatLog { .. } => 0,
                    ItemKind::Mails { .. } => 1,
                    ItemKind::SystemLog { .. } => 2,
                    ItemKind::TextFile(_) => 3,
                    ItemKind::Service { .. } => 4,
                    ItemKind::Firewall(_) => 5,
                };

                kinds[at] = true;
            }
        }
    }

    assert_eq!(kinds, [true; 6]);
}

#[test]
fn should_keep_the_kind_a_mission_sets_for_its_network() {
    let hidden = MissionCatalog::of(MissionKind::Secret).unwrap();
    let kinds: Vec<_> = (0..hidden.len())
        .flat_map(|index| {
            hidden
                .instantiate(index, &mut Random::new(1))
                .unwrap()
                .computers
        })
        .filter_map(|computer| computer.network_kind)
        .collect();

    assert_eq!(kinds.len(), 4);
    assert!(
        kinds
            .iter()
            .all(|kind| *kind == greyhack_behaviour::NetworkKind::Residential)
    );
}

#[test]
fn should_draw_the_npcs_of_a_copy_from_the_random_it_is_given() {
    for kind in LISTED {
        let missions = MissionCatalog::of(kind).unwrap();

        for index in 0..missions.len() {
            let first = missions.instantiate(index, &mut Random::new(7)).unwrap();
            let again = missions.instantiate(index, &mut Random::new(7)).unwrap();
            let other = missions.instantiate(index, &mut Random::new(8)).unwrap();

            assert_eq!(first, again);
            assert_eq!(first.kind, kind);

            if !first.pre_gen_npcs.is_empty() {
                assert_ne!(first.pre_gen_npcs, other.pre_gen_npcs);
            }

            for (at, npc) in first.pre_gen_npcs.iter().enumerate() {
                assert!(!npc.name.is_empty() && !npc.surname.is_empty());
                assert_eq!(npc.forced_pass.is_some(), npc.is_forced_pass);
                assert!(
                    first.pre_gen_npcs[..at]
                        .iter()
                        .all(|earlier| earlier.surname != npc.surname)
                );
            }
        }
    }
}

#[test]
fn should_not_make_a_mission_the_list_does_not_hold() {
    let missions = MissionCatalog::of(MissionKind::ZeroDay).unwrap();

    assert!(missions.instantiate(1, &mut Random::new(1)).is_none());
}
