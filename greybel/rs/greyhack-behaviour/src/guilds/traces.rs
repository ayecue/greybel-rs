//! Traces: when an administrator traces a player and how long it takes (`ActiveTraceSystem`,
//! `PassiveTrace`).

use crate::{LogAction, NetworkKind, TIME_SCALE};

/// Seconds of an active trace per bounce and administrator PC experience point
/// (`ActiveTraceSystem.MAX_TIME_AT`).
pub const ACTIVE_TRACE_SECONDS: u32 = 900;
/// Milliseconds a passive trace waits before it looks at the first company in its queue
/// (`PassiveTrace.NEXT_ADMIN`).
pub const PASSIVE_NEXT_ADMIN_MS: u64 = 30_000;
/// Milliseconds a passive trace waits before each bounce hop (`PassiveTrace.BounceTrace`).
pub const PASSIVE_BOUNCE_MS: u64 = 60_000;
/// Milliseconds divided by the administrator's PC experience before a passive trace reads a log
/// (`PassiveTrace.StartPassiveTrace`).
pub const PASSIVE_BASE_MS: u64 = 900_000;
/// Companies a passive trace queue contains (`PassiveTrace.MAX_TRACES`).
pub const MAX_PASSIVE_TRACES: usize = 32;
/// Game-over strikes that end the game (`PlayerHelperServer.ActiveGameOver`).
pub const MAX_STRIKES: u8 = 3;

/// Whether an action of the player on a computer of a `kind` network with an administrator of
/// `exp_pcs` PC experience starts a trace (`ActiveTraceSystem.ShouldStartTracking`); without an
/// administrator, `exp_pcs` is zero.
#[must_use]
pub fn should_start_tracking(kind: NetworkKind, exp_pcs: u32, action: LogAction) -> bool {
    let (mut triggers, min_exp): (Vec<LogAction>, u32) = match kind {
        NetworkKind::Police | NetworkKind::Bank | NetworkKind::HackShop | NetworkKind::Neurobox => {
            (
                vec![LogAction::Shell, LogAction::Remove, LogAction::Download],
                0,
            )
        }
        NetworkKind::Residential => (vec![LogAction::Remove], 5),
        _ => (vec![LogAction::Remove, LogAction::Download], 0),
    };

    if exp_pcs < min_exp {
        return false;
    }

    if exp_pcs > 7 {
        triggers.push(LogAction::Shell);
    }

    triggers.contains(&action)
}

/// Seconds an active trace through `bounces` bounces lasts against an administrator of `exp_pcs` PC
/// experience (`ActiveTraceSystem.StartActiveTrace`).
#[must_use]
pub fn active_trace_seconds(bounces: u32, exp_pcs: u32) -> f32 {
    bounces as f32 * (ACTIVE_TRACE_SECONDS as f32 / exp_pcs as f32)
}

/// Milliseconds a passive trace waits before it reads the log of a company whose administrator has
/// `exp_pcs` PC experience (`PassiveTrace.StartPassiveTrace`).
#[must_use]
pub fn passive_trace_delay_ms(exp_pcs: u32) -> u64 {
    PASSIVE_BASE_MS / u64::from(exp_pcs.max(1))
}

/// World time an active trace started at `now` ends the player's connection, `None` when it never
/// ends because the administrator has no PC experience (`ActiveTraceSystem.StartActiveTrace`).
#[must_use]
pub fn active_trace_end(now: u64, bounces: u32, exp_pcs: u32) -> Option<u64> {
    let seconds = active_trace_seconds(bounces, exp_pcs).ceil();

    seconds
        .is_finite()
        .then(|| now + seconds as u64 * TIME_SCALE)
}

/// Whether a computer whose administrator is absent goes to the back of the passive trace queue,
/// given the `queue_len` computers left once it is taken off the front (`PassiveTrace.ManageTraces`).
#[must_use]
pub const fn requeues_absent_admin(queue_len: usize) -> bool {
    queue_len < MAX_PASSIVE_TRACES - 1
}

/// Steps of a passive trace (`PassiveTrace.ManageTraces`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PassiveStage {
    /// Waiting to look at the first computer of the queue.
    #[default]
    Wait,
    /// The administrator is tracing the connection, waiting to read the log.
    Read,
    /// Waiting for the next hop back along the connection.
    Bounce,
}

impl PassiveStage {
    /// Number that stands for the stage in storage.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::Wait => 0,
            Self::Read => 1,
            Self::Bounce => 2,
        }
    }

    /// Returns the stage stored as `code`; a number no stage uses is a [`Self::Bounce`].
    #[must_use]
    pub const fn from_code(code: u8) -> Self {
        match code {
            0 => Self::Wait,
            1 => Self::Read,
            _ => Self::Bounce,
        }
    }
}

/// Strikes after the player is traced, as they count and as the game over displays them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StrikeCount {
    /// Strikes the player has afterwards: none once the last one ended the game.
    pub count: u8,
    /// Strikes the game over displays.
    pub visible: u8,
}

/// Counts one more strike on top of `strikes`; the last ends the game and starts the count again
/// (`PlayerHelperServer.ActiveGameOver`).
#[must_use]
pub fn add_strike(strikes: u8) -> StrikeCount {
    let strikes = strikes + 1;

    StrikeCount {
        count: if strikes >= MAX_STRIKES { 0 } else { strikes },
        visible: strikes.min(MAX_STRIKES),
    }
}
