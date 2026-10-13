//! Services and system logs a mission applies after its beats are placed.

use std::collections::HashMap;

use super::beat::Beat;
use super::pending_service::PendingService;
use super::state::State;
use crate::generation::missions::config::EvidenceFunction;
use crate::generation::missions::model::{ItemKind, MissionItem};

pub(super) fn pending_service(beat: i32, function: &EvidenceFunction) -> PendingService {
    PendingService {
        beat,
        service: function.service,
        website: function.http_website_text_id.clone(),
    }
}

/// `ProcMissionBuilder.ResolveRemoteTargetBeat`.
pub(super) fn remote_target(current: &Beat<'_>, next: Option<&Beat<'_>>) -> i32 {
    next.map_or(current.index, |next| next.index)
}

/// `ProcMissionBuilder.ApplyPendingServices`.
pub(super) fn apply_pending_services(state: &mut State<'_>, by_beat: &HashMap<i32, usize>) {
    for queue in std::mem::take(&mut state.services) {
        let Some(&computer) = by_beat.get(&queue.beat) else {
            continue;
        };
        let items = &mut state.mission.computers[computer].items;
        let present = items.iter().any(|item| {
            matches!(&item.kind, ItemKind::Service { service, .. } if *service == queue.service)
        });

        if !present {
            items.push(MissionItem::new(
                &queue.website,
                ItemKind::Service {
                    service: queue.service,
                    allow_duplicates: false,
                },
            ));
        }
    }
}

/// `ProcMissionBuilder.ApplyPendingSystemLogs`.
pub(super) fn apply_pending_system_logs(state: &mut State<'_>, by_beat: &HashMap<i32, usize>) {
    for queue in std::mem::take(&mut state.logs) {
        let Some(&computer) = by_beat.get(&queue.beat) else {
            continue;
        };
        let items = &mut state.mission.computers[computer].items;
        let present = items.iter().any(|item| {
            matches!(
                &item.kind,
                ItemKind::SystemLog { action, port, to_computer_idx, is_receiving_conn }
                    if *action == queue.spec.entry_type
                        && *port == queue.spec.port
                        && *to_computer_idx == queue.to_computer
                        && *is_receiving_conn == queue.spec.is_receiving_conn
            )
        });

        if !present {
            items.push(MissionItem::new(
                "",
                ItemKind::SystemLog {
                    action: queue.spec.entry_type,
                    port: queue.spec.port,
                    to_computer_idx: queue.to_computer,
                    is_receiving_conn: queue.spec.is_receiving_conn,
                },
            ));
        }
    }
}
