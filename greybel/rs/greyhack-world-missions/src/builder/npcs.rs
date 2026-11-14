//! The NPCs of a mission: creating them and their passwords.

use greyhack_behaviour::{Word, hash_password};
use mono_random::Random;

use super::runtime_tags::npc_placeholder;
use super::state::State;
use crate::model::PreGenNpc;
use crate::tags::{next_first_name, next_word};

/// `ProcMissionBuilder.EnsureNpcForBeat`.
pub(super) fn ensure_npc_for_beat(state: &mut State<'_>, beat: i32) -> i32 {
    if let Some(npc) = state.npc_by_beat.get(&beat) {
        return *npc;
    }

    let npc = create_npc(state);

    state.npc_by_beat.insert(beat, npc);

    npc
}

/// `ProcMissionBuilder.CreateProcNpc`.
pub(super) fn create_npc(state: &mut State<'_>) -> i32 {
    let name = next_first_name(state.random);
    let surname = unique_surname(&state.mission.pre_gen_npcs, state.random);

    state.mission.pre_gen_npcs.push(PreGenNpc {
        name,
        surname,
        ..PreGenNpc::default()
    });

    state.mission.pre_gen_npcs.len() as i32 - 1
}

/// `ProcMissionBuilder.GetUniqueSurname`.
pub(super) fn unique_surname(npcs: &[PreGenNpc], random: &mut Random) -> String {
    let unavailable = |surname: &str| npcs.iter().any(|npc| npc.surname == surname);
    let mut surname = next_word(Word::Surname, random);
    let mut tries = 0;

    while unavailable(&surname) && tries < 8 {
        surname = next_word(Word::Surname, random);
        tries += 1;
    }

    if unavailable(&surname) {
        surname = format!("{surname}_{}", npcs.len());
    }

    surname
}

/// `ProcMissionBuilder.EnsureNpcPassword`.
pub(super) fn ensure_npc_password(state: &mut State<'_>, npc: i32) -> String {
    let at = npc as usize;

    state.mission.pre_gen_npcs[at].is_forced_pass = true;

    if state.mission.pre_gen_npcs[at]
        .forced_pass
        .as_deref()
        .is_none_or(str::is_empty)
    {
        let password = next_word(Word::Password, state.random);

        state.mission.pre_gen_npcs[at].forced_pass = Some(password);
    }

    state.mission.pre_gen_npcs[at]
        .forced_pass
        .clone()
        .unwrap_or_default()
}

/// `ProcMissionBuilder.ResolveHistoricalPasswords`: gives the computer of a historical password an account whose
/// current password differs from it.
pub(super) fn resolve_historical_passwords(state: &mut State<'_>) {
    for computer in 0..state.mission.computers.len() {
        for item in 0..state.mission.computers[computer].items.len() {
            let Some(value) = state.mission.computers[computer].items[item]
                .runtime_tags
                .get("$HISTORICAL_PASSWORD")
                .cloned()
            else {
                continue;
            };

            if state.mission.computers[computer].clone_npc_pc_idx < 0 {
                state.mission.computers[computer].clone_npc_pc_idx = create_npc(state);
            }

            let npc = state.mission.computers[computer].clone_npc_pc_idx;
            let current = ensure_npc_password(state, npc);
            let mut value = value;

            if value == current {
                let first = if value.starts_with('A') { "B" } else { "A" };

                value = format!("{first}{}", &value[1..]);
            }

            let tags = &mut state.mission.computers[computer].items[item].runtime_tags;

            tags.insert(
                "$HISTORICAL_PASSWORD_HASH".to_owned(),
                hash_password(&value),
            );
            tags.insert("$HISTORICAL_PASSWORD".to_owned(), value);
            tags.insert("$HISTORICAL_USER".to_owned(), npc_placeholder(npc));
        }
    }
}
