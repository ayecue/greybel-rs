//! Coin mining round ([`Routine::CoinRound`]) that awards one mine job.

use greyhack_behaviour::{cycle_seconds, pick_by_power, pick_machine};

use super::{Routine, RoutineFailure, TickReport};
use crate::world::WorkId;
use crate::{CoinId, MineJobFailure, Result, WorkResult, World};

impl World {
    /// Schedules the next round if none exists once the mining cycle since the last round has
    /// elapsed.
    pub(super) async fn schedule_coin_round(&mut self, coin: CoinId) -> Result<()> {
        let Some(data) = self.db.coin(coin).await? else {
            return Ok(());
        };

        if self.db.coin_round(coin).await?.is_some() {
            return Ok(());
        }

        let at = data.last_cycle_at.map_or(self.time_seconds, |last| {
            last.saturating_add(cycle_seconds(data.cycle_hours))
                .max(self.time_seconds)
        });

        self.schedule(at, Routine::CoinRound { coin }).await?;

        Ok(())
    }

    /// Runs a coin round. One job on a computer with mining power wins via [`World::draw_miner`]
    /// and ends; other jobs continue in the next round one mining cycle later. Jobs with missing
    /// computers fail. The caller ends the current round, so the newly scheduled round is a
    /// separate routine.
    pub(super) async fn run_coin_round(
        &mut self,
        coin: CoinId,
        report: &mut TickReport,
    ) -> Result<()> {
        let Some(mut data) = self.db.coin(coin).await? else {
            report
                .errors
                .push(RoutineFailure::MineJob(MineJobFailure::CoinGone));

            let work = self.coin_jobs(coin).await?;

            return self.fail_work(&work, &MineJobFailure::CoinGone).await;
        };
        let now = self.time_seconds;
        let mut candidates = Vec::new();
        let mut gone = Vec::new();

        for job in self.db.coin_subscribers(coin).await? {
            if self.db.has_computer(job.computer).await? {
                // Grey Hack selects a wallet by total power, then uniformly selects among ALL
                // wallet machines. Zero-power machines remain candidates
                // (`BlockchainSystem.CheckBlocks`).
                let power = self.db.hardware(job.computer).await?.mining_power();

                candidates.push(Candidate {
                    job: job.id,
                    subwallet: job.subwallet,
                    power: f64::from(power),
                });
            } else {
                gone.push(job.id);
            }
        }

        data.last_cycle_at = Some(now);
        self.db.save_coin(&data).await?;

        if !gone.is_empty() {
            report
                .errors
                .push(RoutineFailure::MineJob(MineJobFailure::ComputerUnavailable));
            self.fail_work(&gone, &MineJobFailure::ComputerUnavailable)
                .await?;
        }

        if let Some(winner) = self.draw_miner(coin, candidates).await? {
            let outcome = match self.mine(coin, &winner.subwallet).await? {
                Ok(()) => WorkResult::Complete,
                Err(failure) => {
                    report.errors.push(RoutineFailure::MineJob(failure.clone()));

                    WorkResult::Failure(failure)
                }
            };

            self.end_work(winner.job, outcome).await?;
        }

        if !self.db.coin_subscribers(coin).await?.is_empty() {
            let next = now.saturating_add(cycle_seconds(data.cycle_hours));

            self.schedule(next, Routine::CoinRound { coin }).await?;
        }

        Ok(())
    }

    /// Selects the winning job from `candidates` using `BlockchainSystem.CheckBlocks`: weight
    /// wallets by total machine power, then choose one wallet machine uniformly.
    async fn draw_miner(
        &mut self,
        coin: CoinId,
        candidates: Vec<Candidate>,
    ) -> Result<Option<Candidate>> {
        let mut wallets: Vec<WalletDraw> = Vec::new();

        for candidate in candidates {
            let owner = self
                .db
                .subwallet(coin, &candidate.subwallet)
                .await?
                .map(|subwallet| subwallet.wallet_user)
                .unwrap_or_default();

            match wallets.iter_mut().find(|wallet| wallet.owner == owner) {
                Some(wallet) => {
                    wallet.power += candidate.power;
                    wallet.machines.push(candidate);
                }
                None => wallets.push(WalletDraw {
                    owner,
                    power: candidate.power,
                    machines: vec![candidate],
                }),
            }
        }

        let powers: Vec<f64> = wallets.iter().map(|wallet| wallet.power).collect();
        let Some(choice) = pick_by_power(&mut self.rng, &powers) else {
            return Ok(None);
        };
        let mut machines = wallets.swap_remove(choice).machines;
        let machine = pick_machine(&mut self.rng, machines.len());

        Ok(Some(machines.swap_remove(machine)))
    }
}

/// Mining-round candidate with sub-wallet and computer mining power.
struct Candidate {
    job: WorkId,
    subwallet: String,
    power: f64,
}

/// Wallet candidate with owner, machine jobs, and total mining power.
struct WalletDraw {
    owner: String,
    machines: Vec<Candidate>,
    power: f64,
}
