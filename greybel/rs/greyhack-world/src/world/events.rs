//! Transient world-change notifications for the host. Not persisted.

use crate::{ComputerId, TraceKind, WorkEnd, World};

/// World-change notification.
#[derive(Debug, Clone, PartialEq)]
pub enum WorldEvent {
    /// Computer rebooted and its connections were closed (`HelperScript.CheckBreakConnecion`).
    Reboot { computer: ComputerId },
    /// Scheduled work completed: routine execution/cancellation, mine-job
    /// reward/failure/cancellation, or sniffer capture.
    WorkEnd(WorkEnd),
    /// A trace caught the player: they got strike `strikes` (the third ends the game) for the
    /// computer `computer` (`PlayerHelperServer.ActiveGameOver`).
    GameOver {
        computer: ComputerId,
        trace: TraceKind,
        strikes: u8,
    },
}

impl World {
    /// Returns notifications since the last call: reboots followed by completed work, each in
    /// occurrence order.
    pub fn take_events(&mut self) -> Vec<WorldEvent> {
        let mut events = std::mem::take(&mut self.events);

        events.extend(self.work.take_ends().into_iter().map(WorldEvent::WorkEnd));

        events
    }
}
