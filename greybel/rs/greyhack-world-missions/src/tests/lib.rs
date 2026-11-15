#[path = "support/lib.rs"]
mod support;

use mono_random::Random;
use support::build;

use super::builder::build_mission;
use super::config::{Archetype, ProcMissionsConfig, Source};
use super::model::{ItemKind, TagMap};

#[test]
fn should_read_every_archetype_of_the_game() {
    let config = ProcMissionsConfig::game();

    assert_eq!(config.archetypes.len(), 8);
    assert!(
        config
            .archetypes
            .iter()
            .all(|archetype| archetype.heart.verbs.len() == 4)
    );
    assert!(
        config
            .archetypes
            .iter()
            .flat_map(|archetype| &archetype.heart.verbs)
            .all(|verb| verb.modalities.len() == 4)
    );
}

#[test]
fn should_build_the_same_mission_for_the_same_seed() {
    for seed in 0..20 {
        assert_eq!(
            build(seed, Source::Hackshop).0,
            build(seed, Source::Hackshop).0
        );
    }
}

#[test]
fn should_pick_archetypes_of_the_source() {
    for seed in 0..50 {
        let (_, hackshop) = build(seed, Source::Hackshop);
        let (_, police) = build(seed, Source::Police);

        assert!(matches!(
            hackshop,
            Archetype::Sabotage
                | Archetype::Fraud
                | Archetype::Ransomware
                | Archetype::CorporateEspionage
        ));
        assert!(matches!(
            police,
            Archetype::Extortion
                | Archetype::IdentityTheft
                | Archetype::EvidenceFabrication
                | Archetype::AccessSales
        ));
    }
}

#[test]
fn should_end_every_mission_with_the_closure_computer() {
    for seed in 0..100 {
        let (mission, archetype) = build(seed, Source::Hackshop);
        let last = mission.computers.last().unwrap();

        assert_eq!(last.procedural_archetype, Some(archetype));
        assert_eq!(
            mission
                .computers
                .iter()
                .filter(|computer| computer.procedural_archetype.is_some())
                .count(),
            1
        );
        assert!(last.items.iter().any(|item| item.is_evidence));
        assert!(mission.procedural_level >= 4 && mission.procedural_level <= 15);
        assert!(!mission.starter_text_id.is_empty());
    }
}

#[test]
fn should_fit_the_networks_it_is_given() {
    for seed in 0..100 {
        for networks in [1, 2, 5] {
            let built = build_mission(
                ProcMissionsConfig::game(),
                &mut Random::new(seed),
                networks,
                Source::Police,
            );

            if let Ok((mission, _)) = built {
                assert!(mission.computers.iter().all(|computer| {
                    (usize::try_from(computer.computer_index_net)
                        .expect("network indexes are nonnegative"))
                        < networks
                }));
            }
        }
    }
}

#[test]
fn should_split_a_mission_by_network() {
    for seed in 0..100 {
        let (mission, _) = build(seed, Source::Hackshop);
        let parts = mission.split();
        let computers: usize = parts.iter().map(|part| part.computers.len()).sum();

        assert_eq!(computers, mission.computers.len());
        assert!(parts.iter().all(|part| {
            let network = part.computers[0].computer_index_net;

            part.computers
                .iter()
                .all(|computer| computer.computer_index_net == network)
        }));
    }
}

#[test]
fn should_not_leave_tags_of_historical_passwords_unresolved() {
    for seed in 0..200 {
        let (mission, _) = build(seed, Source::Hackshop);

        for computer in &mission.computers {
            for item in &computer.items {
                if let Some(user) = item.runtime_tags.get("$HISTORICAL_USER") {
                    assert!(computer.clone_npc_pc_idx >= 0);
                    assert_eq!(*user, format!("$NPC_IDX{}", computer.clone_npc_pc_idx));
                    assert!(item.runtime_tags.contains_key("$HISTORICAL_PASSWORD_HASH"));
                }
            }
        }
    }
}

#[test]
fn should_name_every_item_it_builds() {
    for seed in 0..50 {
        let (mission, _) = build(seed, Source::Police);

        for computer in &mission.computers {
            for item in &computer.items {
                match &item.kind {
                    ItemKind::TextFile(file) => {
                        assert!(!file.file_name.is_empty());
                        assert!(!item.id.is_empty());
                    }
                    ItemKind::ChatLog { .. } | ItemKind::Mails { .. } => {
                        assert!(!item.id.is_empty());
                    }
                    ItemKind::SystemLog { .. }
                    | ItemKind::Service { .. }
                    | ItemKind::Firewall(_) => {}
                }
            }
        }
    }
}

#[test]
fn should_seed_from_the_address_bytes_in_order() {
    assert_eq!(super::seed_from_ip("1.2.3.4"), 0x0403_0201);
    assert_eq!(super::seed_from_ip("not an address"), 0);
}

#[test]
fn should_draw_consistent_digit_checksums() {
    for seed in 0..50 {
        let mut tags = TagMap::new();

        super::tags::digit_checksum(&mut tags, &mut Random::new(seed));

        let code = &tags["$DIGIT_CODE"];
        let sum: u32 = code.bytes().map(|digit| u32::from(digit - b'0')).sum();

        assert_eq!(tags["$DIGIT_CHECKSUM"], (sum % 10).to_string());
        assert_eq!(tags["$DIGIT_PARTIAL"].matches('?').count(), 1);
        assert_ne!(tags["$DIGIT_DECOY1"], tags["$DIGIT_DECOY2"]);
        assert_ne!(&tags["$DIGIT_DECOY1"], code);
    }
}

#[test]
fn should_draw_file_timelines_in_the_month_before_the_start() {
    let mut tags = TagMap::new();

    super::tags::file_timeline(&mut tags, &mut Random::new(5));

    assert!(
        tags["$FILE_TIMELINE_START_A"].contains("/Dec/1999")
            || tags["$FILE_TIMELINE_START_A"].contains("/Jan/2000")
    );
    assert_eq!(tags["$FILE_TIMELINE_PASSWORD"].len(), 12);
    assert_eq!(tags["$FILE_TIMELINE_COVER_DATE"].len(), 8);
}

#[test]
fn should_keep_clock_offsets_apart() {
    for seed in 0..50 {
        let tags = super::tags::clock_offset(&mut Random::new(seed));

        assert_ne!(tags["$CLOCK_EVENT_LOCAL"], tags["$CLOCK_EVENT_SERVER"]);
        assert_eq!(tags["$CLOCK_EVENT_LOCAL"].len(), 5);
    }
}
