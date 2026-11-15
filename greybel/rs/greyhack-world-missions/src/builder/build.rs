//! Building a mission: the attempts at a candidate that fits the networks.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::LazyLock;

use mono_random::Random;

use super::attachments::{apply_pending_services, apply_pending_system_logs};
use super::beat::Beat;
use super::build_error::BuildError;
use super::items::{
    build_item_from_option, closure_item, collect_cross_info_items, support_chat_log_item,
};
use super::npcs::resolve_historical_passwords;
use super::runtime_tags::resolve_runtime_tags;
use super::selection::{
    assign_local_indexes, pick_closure, pick_starter, select_archetype, select_heart_beats,
};
use super::state::State;
use crate::config::{
    Archetype, EvidenceType, Modality, ProcMissionsConfig, Source, TemplateOption,
};
use crate::model::{ComputerMission, Mission, MissionItem, TagMap};

/// Highest network index of a mission (`ProcMissionBuilder.MaxNetworkIndex`).
pub(super) const MAX_NETWORK_INDEX: i32 = 9;

/// Candidates tried for a mission that fits the networks (`ProcMissionBuilder.MaxGenerationAttempts`).
pub(super) const MAX_ATTEMPTS: usize = 32;

pub(super) static EMPTY_OPTION: LazyLock<TemplateOption> = LazyLock::new(TemplateOption::default);

pub(super) type TagStore = Rc<RefCell<HashMap<EvidenceType, TagMap>>>;

/// `ProcMissionBuilder.BuildMission`: a mission of `source` that fits `available_networks` public networks, and its
/// archetype.
pub fn build_mission(
    config: &ProcMissionsConfig,
    random: &mut Random,
    available_networks: usize,
    source: Source,
) -> Result<(Mission, Archetype), BuildError> {
    for _ in 0..MAX_ATTEMPTS {
        let (mission, archetype) = build_candidate(config, random, source)?;

        if required_networks(&mission) <= available_networks {
            return Ok((mission, archetype));
        }
    }

    Err(BuildError::NetworkLimit)
}

/// `ProcMissionBuilder.GetRequiredNetworkCount`.
pub(super) fn required_networks(mission: &Mission) -> usize {
    let highest = mission
        .computers
        .iter()
        .map(|computer| computer.computer_index_net)
        .fold(mission.closure_network_index, i32::max);

    usize::try_from((highest + 1).max(0)).expect("the clamped index is nonnegative")
}

pub(super) fn build_candidate(
    config: &ProcMissionsConfig,
    random: &mut Random,
    source: Source,
) -> Result<(Mission, Archetype), BuildError> {
    let level = config.procedural_level(random);
    let archetype_config = select_archetype(config, random, source)?;
    let hearts = config.hearts_count(level);
    let starter = pick_starter(archetype_config, random)?;
    let mut beats = select_heart_beats(archetype_config, hearts, random);
    let next_local = assign_local_indexes(&mut beats);
    let last = beats.last().ok_or(BuildError::MissingTemplate)?;
    let mut closure_network = last.network;

    if last.option.contains_evidence(EvidenceType::NextNodeRemote) {
        closure_network = (closure_network + 1).min(MAX_NETWORK_INDEX);
    }

    let closure = Beat {
        index: i32::try_from(beats.len()).expect("value fits the target integer type"),
        network: closure_network,
        local: next_local.get(&closure_network).copied().unwrap_or(0),
        modality: Modality::File,
        option: &EMPTY_OPTION,
    };
    let mut state = State {
        mission: Mission {
            procedural_level: level,
            starter_text_id: starter.text_id.clone(),
            ..Mission::default()
        },
        npc_by_beat: HashMap::new(),
        global_npc: -1,
        global_string: None,
        services: Vec::new(),
        logs: Vec::new(),
        random,
    };
    let mut beat_tags: Vec<TagMap> = Vec::new();
    let mut common: Vec<Option<TagStore>> = vec![None; beats.len() + 1];
    let mut extras: HashMap<i32, Vec<MissionItem>> = HashMap::new();

    for (index, beat) in beats.iter().enumerate() {
        let next = beats.get(index + 1).unwrap_or(&closure);

        beat_tags.push(resolve_runtime_tags(
            &mut state,
            beat,
            Some(next),
            beat.option,
            &mut common[index],
        )?);
        collect_cross_info_items(
            &mut state,
            &beats,
            index,
            &closure,
            &mut common,
            &mut extras,
        )?;
    }

    let mut by_beat: HashMap<i32, usize> = HashMap::new();

    for (index, beat) in beats.iter().enumerate() {
        let mut computer = ComputerMission::new(beat.network);

        if let Some(npc) = state.npc_by_beat.get(&beat.index) {
            computer.clone_npc_pc_idx = *npc;
        }

        if let Some(mut item) = build_item_from_option(beat, state.random) {
            item.runtime_tags.clone_from(&beat_tags[index]);
            computer.items.push(item);
        }

        if let Some(item) = support_chat_log_item(beat, &beat_tags[index]) {
            computer.items.push(item);
        }

        if let Some(items) = extras.remove(&beat.index) {
            computer.items.extend(items);
        }

        by_beat.insert(beat.index, state.mission.computers.len());
        state.mission.computers.push(computer);
    }

    let closure_entry = pick_closure(archetype_config, starter, state.random)?;
    let closure_index = beats.len();
    let closure_tags = resolve_runtime_tags(
        &mut state,
        &closure,
        None,
        &closure_entry.option,
        &mut common[closure_index],
    )?;
    let mut computer = ComputerMission::new(closure_network);

    computer.procedural_archetype = Some(archetype_config.archetype);
    computer
        .items
        .push(closure_item(closure_entry, closure_tags, state.random));

    if let Some(items) = extras.remove(&closure.index) {
        computer.items.extend(items);
    }

    if let Some(npc) = state.npc_by_beat.get(&closure.index) {
        computer.clone_npc_pc_idx = *npc;
    }

    by_beat.insert(closure.index, state.mission.computers.len());
    state.mission.computers.push(computer);
    state.mission.closure_network_index = closure.network;
    state.mission.closure_local_index = closure.local;
    state.mission.procedural_global_npc_index = state.global_npc;
    apply_pending_services(&mut state, &by_beat);
    apply_pending_system_logs(&mut state, &by_beat);
    resolve_historical_passwords(&mut state);

    Ok((state.mission, archetype_config.archetype))
}
