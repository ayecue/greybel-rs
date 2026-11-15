//! Choosing the archetype, starter, heart beats, and closure of a mission.

use std::collections::HashMap;

use mono_random::Random;

use super::beat::Beat;
use super::build::MAX_NETWORK_INDEX;
use super::build_error::BuildError;
use super::preset::Preset;
use crate::config::{
    ArchetypeConfig, ClosureEntry, EvidenceType, Modality, NextMode, PayloadType,
    ProcMissionsConfig, Source, TemplateOption,
};

pub(super) type Candidate<'c> = (Modality, &'c TemplateOption);

/// `ProcMissionBuilder.SelectArchetype`.
pub(super) fn select_archetype<'c>(
    config: &'c ProcMissionsConfig,
    random: &mut Random,
    source: Source,
) -> Result<&'c ArchetypeConfig, BuildError> {
    let candidates: Vec<&ArchetypeConfig> = config
        .archetypes
        .iter()
        .filter(|archetype| archetype.source == source && has_any_heart_option(archetype))
        .collect();

    if candidates.is_empty() {
        return Err(BuildError::NoTemplates);
    }

    Ok(candidates[random.next_below(candidates.len())])
}

pub(super) fn has_any_heart_option(archetype: &ArchetypeConfig) -> bool {
    archetype.heart.verbs.iter().any(|verb| {
        verb.modalities
            .iter()
            .any(|modality| !modality.options.is_empty())
    })
}

/// `ProcMissionBuilder.PickStarter`.
pub(super) fn pick_starter<'c>(
    archetype: &'c ArchetypeConfig,
    random: &mut Random,
) -> Result<&'c TemplateOption, BuildError> {
    if archetype.starters.is_empty() {
        return Err(BuildError::MissingTemplate);
    }

    Ok(&archetype.starters[random.next_below(archetype.starters.len())])
}

/// `ProcMissionBuilder.PickClosure`: a closure of the group of `starter`.
pub(super) fn pick_closure<'c>(
    archetype: &'c ArchetypeConfig,
    starter: &TemplateOption,
    random: &mut Random,
) -> Result<&'c ClosureEntry, BuildError> {
    let group = starter.group.trim();
    let entries: Vec<&ClosureEntry> = archetype
        .closure
        .entries
        .iter()
        .filter(|entry| entry.option.group.trim() == group)
        .collect();

    if entries.is_empty() {
        return Err(BuildError::MissingTemplate);
    }

    Ok(entries[random.next_below(entries.len())])
}

/// `ProcMissionBuilder.SelectHeartBeats`: the verb, modality and option of each beat, all on network 0 until a beat
/// points at the next network.
pub(super) fn select_heart_beats<'c>(
    archetype: &'c ArchetypeConfig,
    hearts: i32,
    random: &mut Random,
) -> Vec<Beat<'c>> {
    let mut beats: Vec<Beat<'c>> = Vec::new();
    let mut previous: Option<Modality> = None;
    let mut preset: Option<Preset<'c>> = None;
    let mut network = 0;

    for index in 0..hearts {
        let beat = if let Some(preset) = preset.take() {
            Beat {
                index: i32::try_from(beats.len()).expect("value fits the target integer type"),
                network,
                local: 0,
                modality: preset.modality,
                option: preset.option,
            }
        } else {
            let last = index == hearts - 1;
            let mut strict = candidates(archetype, previous, true, last);
            let mut loose = candidates(archetype, previous, false, last);

            if last && strict.is_empty() && loose.is_empty() {
                strict = candidates(archetype, previous, true, false);
                loose = candidates(archetype, previous, false, false);
            }

            let pool = if strict.is_empty() { loose } else { strict };

            if pool.is_empty() {
                break;
            }

            let (modality, option) = pool[random.next_below(pool.len())];

            Beat {
                index: i32::try_from(beats.len()).expect("value fits the target integer type"),
                network,
                local: 0,
                modality,
                option,
            }
        };

        previous = Some(beat.modality);

        if beat.option.contains_evidence(EvidenceType::NextNodeRemote) {
            network = (network + 1).min(MAX_NETWORK_INDEX);
        }

        if beat.modality == Modality::CrossInfo
            && let Some(spec) = &beat.option.cross_info_spec
            && spec.next_mode == NextMode::ReplaceNextBeat
        {
            preset = Some(Preset {
                modality: match spec.next_type {
                    PayloadType::File => Modality::File,
                    PayloadType::ChatLog => Modality::ChatLog,
                },
                option: &spec.next_option,
            });
        }

        beats.push(beat);
    }

    beats
}

/// `ProcMissionBuilder.BuildCandidates`.
pub(super) fn candidates<'c>(
    archetype: &'c ArchetypeConfig,
    previous: Option<Modality>,
    no_consecutive: bool,
    exclude_cross_info: bool,
) -> Vec<Candidate<'c>> {
    let mut list = Vec::new();

    for verb in &archetype.heart.verbs {
        for modality in &verb.modalities {
            if (no_consecutive && previous == Some(modality.modality))
                || (exclude_cross_info && modality.modality == Modality::CrossInfo)
                || modality.options.is_empty()
            {
                continue;
            }

            list.extend(
                modality
                    .options
                    .iter()
                    .map(|option| (modality.modality, option)),
            );
        }
    }

    list
}

/// `ProcMissionBuilder.AssignLocalIndexes`: numbers the beats of each network and returns how many each has.
pub(super) fn assign_local_indexes(beats: &mut [Beat<'_>]) -> HashMap<i32, i32> {
    let mut next: HashMap<i32, i32> = HashMap::new();

    for beat in beats {
        let local = next.get(&beat.network).copied().unwrap_or(0);

        beat.local = local;
        next.insert(beat.network, local + 1);
    }

    next
}
