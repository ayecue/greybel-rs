//! Mining work and its domain-owned failures.

use greyhack_world_modules::WorkId;

use crate::{CoinId, ComputerId};

/// Mining failures accumulated while economy routines run.
#[derive(Debug, Default)]
pub struct MineJobReport {
    /// Failures to append to the current tick report.
    pub errors: Vec<MineJobFailure>,
}

/// One running mine job in a program tree.
#[derive(Debug, Default)]
pub struct MineJobState {
    /// Active work identifier, if mining is running.
    pub work: Option<WorkId>,
}

/// Sub-wallet mining job on a computer, participating in coin rounds until it wins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MineJob {
    /// Work identifier.
    pub id: WorkId,
    /// Coin being mined.
    pub coin: CoinId,
    /// Sub-wallet account.
    pub subwallet: String,
    /// Mining computer.
    pub computer: ComputerId,
}

/// Mining start and reward errors (`SubWalletIntrinsics.mining`, `BlockchainSystem.CheckBlocks`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MineJobFailure {
    /// The mining computer no longer exists.
    ComputerUnavailable,
    /// The coin does not exist.
    CoinGone,
    /// The named coin has been fully mined.
    AllMined(String),
    /// The sub-wallet does not exist.
    SubwalletGone,
    /// The sub-wallet already mines this coin on the computer.
    SecondMineJob,
    /// The computer GPU health is below 5.
    DamagedGpu,
    /// The GPU has health below 40 and failed its floating-point self-test.
    CorruptedGpu,
}
