//! The missions the game authors (`MissionGen.missions`, read from the `MissionHidden`, `MissionKarma`,
//! `MissionKarmaBlack`, `MissionKarmaWhite` and `MissionZeroDay` texts) and the copy of one the game makes for a
//! group of networks (`MissionConfig.Mission(Mission, Random)`).

mod raw;

use std::sync::LazyLock;

use greyhack_behaviour::{MissionKind, Word};
use greyhack_world_assets::MissionList;
use mono_random::Random;

use crate::model::{Mission, PreGenNpc};
use crate::tags::{next_first_name, next_word};

/// The authored missions of one kind of mission.
pub struct MissionCatalog {
    missions: Vec<Mission>,
}

impl MissionCatalog {
    /// The missions of `kind` as the game's lists contain them; `None` for the kinds that are not in the lists.
    pub fn of(kind: MissionKind) -> Option<&'static Self> {
        static SECRET: LazyLock<MissionCatalog> =
            LazyLock::new(|| MissionCatalog::read(MissionList::Secret, MissionKind::Secret));
        static GREY: LazyLock<MissionCatalog> =
            LazyLock::new(|| MissionCatalog::read(MissionList::Grey, MissionKind::Grey));
        static BLACK: LazyLock<MissionCatalog> =
            LazyLock::new(|| MissionCatalog::read(MissionList::Black, MissionKind::Black));
        static WHITE: LazyLock<MissionCatalog> =
            LazyLock::new(|| MissionCatalog::read(MissionList::White, MissionKind::White));
        static ZERO_DAY: LazyLock<MissionCatalog> =
            LazyLock::new(|| MissionCatalog::read(MissionList::ZeroDay, MissionKind::ZeroDay));

        match kind {
            MissionKind::Secret => Some(&SECRET),
            MissionKind::Grey => Some(&GREY),
            MissionKind::Black => Some(&BLACK),
            MissionKind::White => Some(&WHITE),
            MissionKind::ZeroDay => Some(&ZERO_DAY),
            _ => None,
        }
    }

    fn read(list: MissionList, kind: MissionKind) -> Self {
        let missions: Vec<raw::RawMission> =
            serde_json::from_str(list.text()).expect("the bundled authored missions are valid");

        Self {
            missions: missions
                .into_iter()
                .map(|mission| mission.into_mission(kind))
                .collect(),
        }
    }

    pub fn len(&self) -> usize {
        self.missions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.missions.is_empty()
    }

    /// `MissionGen.GetMission` before `SplitMission`: a copy of mission `index` with NPCs drawn from `random`
    /// (`MissionConfig.Mission(Mission, Random)`); `None` where the list has no such mission.
    pub fn instantiate(&self, index: usize, random: &mut Random) -> Option<Mission> {
        let template = self.missions.get(index)?;
        let mut mission = Mission {
            pre_gen_npcs: Vec::new(),
            ..template.clone()
        };

        for original in &template.pre_gen_npcs {
            let mut npc = draw_npc(original, random);

            while mission
                .pre_gen_npcs
                .iter()
                .any(|other| other.surname == npc.surname)
            {
                npc.surname = next_word(Word::Surname, random);
            }

            mission.pre_gen_npcs.push(npc);
        }

        Some(mission)
    }
}

/// `PreGenNpc.PreGenNpc(Random, PreGenNpc)`: a first name, a surname, and a password where the NPC forces one.
fn draw_npc(original: &PreGenNpc, random: &mut Random) -> PreGenNpc {
    let name = next_first_name(random);
    let surname = next_word(Word::Surname, random);
    let forced_pass = original
        .is_forced_pass
        .then(|| next_word(Word::Password, random));

    PreGenNpc {
        name,
        surname,
        is_victim: original.is_victim,
        is_guilty: original.is_guilty,
        is_forced_pass: original.is_forced_pass,
        forced_pass,
    }
}
