//! The tags a mission resolves while it is built, and their placeholders.

use greyhack_behaviour::hash_password;

use super::attachments::{pending_service, remote_target};
use super::beat::Beat;
use super::build::{MAX_NETWORK_INDEX, TagStore};
use super::build_error::BuildError;
use super::npcs::{create_npc, ensure_npc_for_beat, ensure_npc_password};
use super::pending_system_log::PendingSystemLog;
use super::state::State;
use crate::generation::missions::config::{EvidenceType, TemplateOption};
use crate::generation::missions::model::TagMap;
use crate::generation::missions::tags;
use crate::generation::missions::tags::{between, random_alphanumeric};

/// `ProcMissionBuilder.ResolveRuntimeTags`: the tags of an item of beat `current`, whose next beat is `next`.
#[allow(clippy::too_many_lines)]
pub(super) fn resolve_runtime_tags(
    state: &mut State<'_>,
    current: &Beat<'_>,
    next: Option<&Beat<'_>>,
    option: &TemplateOption,
    common: &mut Option<TagStore>,
) -> Result<TagMap, BuildError> {
    let mut tags = TagMap::new();
    let local = option.contains_evidence(EvidenceType::NextNodeLocal);
    let remote = option.contains_evidence(EvidenceType::NextNodeRemote);
    let same_network = if local {
        next.filter(|next| next.network == current.network)
    } else {
        None
    };

    if local {
        let target = same_network.ok_or(BuildError::MissingBeat)?;

        tags.insert("$IP_LOCAL".to_owned(), local_placeholder(target.local));
    }

    if remote {
        let mut network = next.map_or(current.network, |next| next.network);

        if network <= current.network {
            network = (current.network + 1).min(MAX_NETWORK_INDEX);
        }

        tags.insert("$IP_REMOTE".to_owned(), public_placeholder(network));
        tags.insert(
            "$IP_REMLOCAL".to_owned(),
            local_placeholder(next.map_or(current.local, |next| next.local)),
        );
    }

    let (mut public, mut local_ips, mut strings, mut randoms) = (0, 0, 0, 0);

    for function in &option.evidence {
        match function.kind {
            EvidenceType::NpcNameGlobal => {
                if state.global_npc < 0 {
                    state.global_npc = create_npc(state);
                }

                tags.insert(
                    "$NPC_NAME_GLOBAL".to_owned(),
                    npc_placeholder(state.global_npc),
                );

                if function.force_account_on_current_node {
                    state.npc_by_beat.insert(current.index, state.global_npc);
                }

                if function.force_account_on_next_node
                    && let Some(next) = next
                {
                    state.npc_by_beat.insert(next.index, state.global_npc);
                }
            }
            EvidenceType::NpcNameLocal => {
                let target = same_network.ok_or(BuildError::MissingBeat)?;
                let npc = ensure_npc_for_beat(state, target.index);

                tags.insert("$NPC_NAME_LOCAL".to_owned(), npc_placeholder(npc));
            }
            EvidenceType::NextNpcPassword | EvidenceType::NextNpcEncryptedPassword => {
                let beat = remote_target(current, next);
                let npc = ensure_npc_for_beat(state, beat);
                let password = ensure_npc_password(state, npc);

                if function.kind == EvidenceType::NextNpcEncryptedPassword {
                    tags.insert(
                        "$NEXT_NPC_ENCRYPTED_PASSWORD".to_owned(),
                        hash_password(&password),
                    );
                } else {
                    tags.insert("$NEXT_NPC_PASSWORD".to_owned(), password);
                }
            }
            EvidenceType::NextServiceLocal => {
                let target = same_network.ok_or(BuildError::MissingBeat)?;

                state.services.push(pending_service(target.index, function));
            }
            EvidenceType::NextServiceRemote => {
                state
                    .services
                    .push(pending_service(remote_target(current, next), function));
            }
            EvidenceType::NextSystemLog => {
                let mut to_computer = current.network;

                if let Some(next) = next
                    && next.option.text_id.is_empty()
                    && next.network == to_computer
                    && to_computer > 0
                {
                    to_computer -= 1;
                }

                state.logs.push(PendingSystemLog {
                    beat: remote_target(current, next),
                    to_computer,
                    spec: function.system_log_spec.clone(),
                });
            }
            EvidenceType::ActualSystemLog => {
                state.logs.push(PendingSystemLog {
                    beat: current.index,
                    to_computer: next.map_or(current.network, |next| next.network),
                    spec: function.system_log_spec.clone(),
                });
            }
            EvidenceType::RandomPublicIp => {
                public += 1;
                tags.insert(
                    indexed_tag("$RANDOM_PUBLIC_IP", public),
                    tags::random_public_ip(state.random),
                );
            }
            EvidenceType::RandomLocalIp => {
                local_ips += 1;
                tags.insert(
                    indexed_tag("$RANDOM_LOCAL_IP", local_ips),
                    tags::random_local_ip(state.random),
                );
            }
            EvidenceType::RandomStringLocal => {
                strings += 1;
                tags.insert(
                    indexed_tag("$RANDOM_STRING_LOCAL", strings),
                    random_alphanumeric(state.random, 8),
                );
            }
            EvidenceType::HistoricalPassword => {
                tags.insert(
                    "$HISTORICAL_PASSWORD".to_owned(),
                    random_alphanumeric(state.random, 8),
                );
            }
            EvidenceType::RandomStringGlobal => {
                if state.global_string.is_none() {
                    state.global_string = Some(random_alphanumeric(state.random, 8));
                }

                tags.insert(
                    "$RANDOM_STRING_GLOBAL".to_owned(),
                    state.global_string.clone().unwrap_or_default(),
                );
            }
            EvidenceType::NpcNameRandom => {
                randoms += 1;

                let npc = create_npc(state);

                tags.insert(
                    indexed_tag("$NPC_NAME_RANDOM", randoms),
                    npc_placeholder(npc),
                );
            }
            EvidenceType::TransferBalance => tags::transfer_balance(&mut tags, state.random),
            EvidenceType::SplitPayment => tags::split_payment(&mut tags, state.random),
            EvidenceType::BlockCoverage => tags::block_coverage(&mut tags, state.random),
            EvidenceType::DigitChecksum => tags::digit_checksum(&mut tags, state.random),
            EvidenceType::FileTimeline => tags::file_timeline(&mut tags, state.random),
            EvidenceType::CircularLog => {
                tags.insert(
                    "$CIRCULAR_LOG_NEXT".to_owned(),
                    between(state.random, 0, 5).to_string(),
                );
            }
            EvidenceType::RevisionBoundary => tags::revision_boundary(&mut tags, state.random),
            EvidenceType::ClockOffset | EvidenceType::OctetPermutation => {
                let map = common.get_or_insert_with(TagStore::default);
                let current = map.borrow().get(&function.kind).cloned();
                let values = match current {
                    Some(values) => values,
                    None => {
                        let values = if function.kind == EvidenceType::ClockOffset {
                            tags::clock_offset(state.random)
                        } else {
                            tags::octet_permutation(state.random)
                        };

                        map.borrow_mut().insert(function.kind, values.clone());

                        values
                    }
                };

                tags.extend(values);
            }
            EvidenceType::NextNodeLocal | EvidenceType::NextNodeRemote => {}
        }
    }

    Ok(tags)
}

pub(super) fn indexed_tag(base: &str, occurrence: i32) -> String {
    if occurrence <= 1 {
        base.to_owned()
    } else {
        format!("{base}{occurrence}")
    }
}

pub(super) fn public_placeholder(network: i32) -> String {
    format!("$IP_IDX{}", network.max(0))
}

pub(super) fn local_placeholder(local: i32) -> String {
    format!("$IP_LOCAL_IDX{}", local.max(0))
}

pub(super) fn npc_placeholder(npc: i32) -> String {
    format!("$NPC_IDX{}", npc.max(0))
}
