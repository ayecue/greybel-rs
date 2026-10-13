//! Program-visible work: routines, mining subscribers, and sniffers identified by row-derived work
//! IDs, plus completions for the current cycle.

use crate::{CoinId, ComputerId, MineJobFailure, SnifferPacket};

/// Work identifier: routine row ID or mine-job/sniffer ID from [`Work`].
pub(crate) type WorkId = u64;

/// Sub-wallet mining job on a computer, participating in coin rounds until it wins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MineJob {
    pub id: WorkId,
    pub coin: CoinId,
    pub subwallet: String,
    pub computer: ComputerId,
}

/// Completed work and its result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkEnd {
    pub id: WorkId,
    pub result: WorkResult,
}

/// Work completion result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkResult {
    /// A routine ran or a mine job won its coin reward.
    Complete,
    /// The mine job cannot run.
    Failure(MineJobFailure),
    /// Packet captured by a sniffer.
    Capture(SnifferPacket),
    /// The waiter canceled the work.
    Cancel,
}

/// Completed work since the last retrieval.
#[derive(Debug, Default)]
pub(crate) struct Work {
    ends: Vec<WorkEnd>,
}

impl Work {
    pub(crate) fn push_end(&mut self, id: WorkId, result: WorkResult) {
        self.ends.push(WorkEnd { id, result });
    }

    /// Returns completed work since the last call, in completion order.
    pub(crate) fn take_ends(&mut self) -> Vec<WorkEnd> {
        std::mem::take(&mut self.ends)
    }
}
