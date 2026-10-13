//! Routes plugin notifications to terminals, contexts, or builds from tagged `pid` fields and context/build IDs.

use duplex_core::{FieldAccess, FieldTagContainer, ValueRef};

use super::{GreybelNotification, PID_TAG, WindowPidKind};

/// Synthetic PID for internal bash and notifications without windows, matching TypeScript
/// `GreyHackResponse.getPID`.
const INTERNAL_BASH_PID: i32 = -10;
const CONTEXT_ID: &str = "context_id";
const BUILD_ID: &str = "build_id";

impl GreybelNotification {
    /// Extracts the message's `context_id` text, if present.
    #[must_use]
    pub fn context_id(&self) -> Option<&str> {
        self.field(CONTEXT_ID)?.text()
    }

    /// Extracts the message's `build_id` text, if present.
    #[must_use]
    pub fn build_id(&self) -> Option<&str> {
        self.field(BUILD_ID)?.text()
    }

    /// Decodes the `pid`-tagged field as window PIDs.
    #[must_use]
    pub fn window_pids(&self) -> WindowPidKind<'_> {
        let Some(window) = FieldTagContainer::with_tag(self.fields(), PID_TAG) else {
            return WindowPidKind::None;
        };

        match self.field(window.name) {
            Some(ValueRef::Integer(pid)) => {
                i32::try_from(pid).map_or(WindowPidKind::None, WindowPidKind::One)
            }
            Some(ValueRef::List(pids)) => WindowPidKind::List(
                pids.iter()
                    .filter_map(|pid| pid.integer().and_then(|pid| i32::try_from(pid).ok()))
                    .collect(),
            ),
            Some(ValueRef::Bytes(payload)) => WindowPidKind::Payload(payload),
            _ => WindowPidKind::None,
        }
    }

    /// Checks whether the notification targets `pid`; missing PIDs select the internal bash PID.
    #[must_use]
    pub fn matches_pid(&self, pid: i32) -> bool {
        match self.window_pids() {
            WindowPidKind::None => pid == INTERNAL_BASH_PID,
            WindowPidKind::One(own) => own == pid || (pid == INTERNAL_BASH_PID && own == -1),
            pids => pids.contains(pid),
        }
    }

    /// Checks for a `context_id` or a target of `terminal_pid`.
    #[must_use]
    pub fn belongs_to_context(&self, context_id: &str, terminal_pid: i32) -> bool {
        self.context_id() == Some(context_id) || self.matches_pid(terminal_pid)
    }

    /// Checks whether the message contains `build_id`.
    #[must_use]
    pub fn belongs_to_build(&self, build_id: &str) -> bool {
        self.build_id() == Some(build_id)
    }
}
