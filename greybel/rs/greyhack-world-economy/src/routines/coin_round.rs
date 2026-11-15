//! Coin mining round owned by the economy domain.

use std::rc::Rc;

use greyhack_behaviour::{cycle_seconds, pick_by_power, pick_machine};
use greyhack_world_core::{ComputerService, WorldRef};
use greyhack_world_modules::WorkId;
use greyhack_world_routines::{
    Result as RoutineResult, Routine, RoutineError, RoutineId, Scheduler,
};
use greyhack_world_storage::TableSet;
use greyhack_world_storage::sql::{execute, fetch_value};
use turso::{Connection, params};

use crate::routines::MineJobReport;
use crate::storage::{RoutineRepository, schema};
use crate::{CoinId, CoinService, MineJobFailure, MineJobService, WalletService};

/// A coin mining round that rewards one mine job.
pub struct CoinRound {
    world: WorldRef,
    computers: greyhack_world_core::Dep<ComputerService>,
    coins: greyhack_world_core::Dep<CoinService>,
    wallets: greyhack_world_core::Dep<WalletService>,
    mine_jobs: greyhack_world_core::Dep<MineJobService>,
    routines: Rc<RoutineRepository>,
    scheduler: Rc<Scheduler>,
}

pub(crate) struct CoinRoundSupport {
    pub(crate) computers: greyhack_world_core::Dep<ComputerService>,
    pub(crate) coins: greyhack_world_core::Dep<CoinService>,
    pub(crate) wallets: greyhack_world_core::Dep<WalletService>,
    pub(crate) mine_jobs: greyhack_world_core::Dep<MineJobService>,
    pub(crate) routines: Rc<RoutineRepository>,
    pub(crate) scheduler: Rc<Scheduler>,
}

impl CoinRound {
    pub(crate) fn new(world: &WorldRef, dependencies: CoinRoundSupport) -> Self {
        Self {
            world: world.clone(),
            computers: dependencies.computers,
            coins: dependencies.coins,
            wallets: dependencies.wallets,
            mine_jobs: dependencies.mine_jobs,
            routines: dependencies.routines,
            scheduler: dependencies.scheduler,
        }
    }

    pub(crate) async fn schedule(&self, coin: CoinId) -> crate::Result<()> {
        let Some(data) = self.coins.coin(coin).await? else {
            return Ok(());
        };

        if self.routines.coin_round(coin).await?.is_some() {
            return Ok(());
        }

        let at = data
            .last_cycle_at
            .map_or(self.world.time_seconds(), |last| {
                last.saturating_add(cycle_seconds(data.cycle_hours))
                    .max(self.world.time_seconds())
            });

        self.schedule_at(at, coin).await?;

        Ok(())
    }

    /// Schedules a mining round at an explicit world second.
    pub async fn schedule_at(&self, at: u64, coin: CoinId) -> crate::Result<RoutineId> {
        Ok(self
            .scheduler
            .schedule::<Self>(&self.world.db().connection(), at, &coin)
            .await?)
    }

    async fn execute(&self, coin: CoinId) -> crate::Result<()> {
        let Some(mut data) = self.coins.coin(coin).await? else {
            self.world
                .world_mut()
                .state_mut::<MineJobReport>()
                .errors
                .push(MineJobFailure::CoinMissing);

            let work = self.coin_jobs(coin).await?;

            return self
                .mine_jobs
                .fail(&work, &MineJobFailure::CoinMissing)
                .await;
        };
        let now = self.world.time_seconds();
        let mut candidates = Vec::new();
        let mut unavailable = Vec::new();

        for job in self.mine_jobs.coin_subscribers(coin).await? {
            if self.computers.has_computer(job.computer).await? {
                // The game selects a wallet by total power, then uniformly among all its machines.
                let power = self.computers.hardware(job.computer).await?.mine_power();

                candidates.push(Candidate {
                    job: job.id,
                    subwallet: job.subwallet,
                    power: f64::from(power),
                });
            } else {
                unavailable.push(job.id);
            }
        }

        data.last_cycle_at = Some(now);
        self.coins.save_coin(&data).await?;
        if !unavailable.is_empty() {
            self.world
                .world_mut()
                .state_mut::<MineJobReport>()
                .errors
                .push(MineJobFailure::ComputerUnavailable);
            self.mine_jobs
                .fail(&unavailable, &MineJobFailure::ComputerUnavailable)
                .await?;
        }

        if let Some(winner) = self.draw_miner(coin, candidates).await? {
            let outcome = match self.coins.mine(coin, &winner.subwallet).await? {
                Ok(()) => Ok(()),
                Err(failure) => {
                    self.world
                        .world_mut()
                        .state_mut::<MineJobReport>()
                        .errors
                        .push(failure.clone());

                    Err(failure)
                }
            };

            self.mine_jobs.end(winner.job, outcome).await?;
        }

        if !self.mine_jobs.coin_subscribers(coin).await?.is_empty() {
            let next = now.saturating_add(cycle_seconds(data.cycle_hours));

            self.scheduler
                .schedule::<Self>(&self.world.db().connection(), next, &coin)
                .await?;
        }

        Ok(())
    }

    async fn draw_miner(
        &self,
        coin: CoinId,
        candidates: Vec<Candidate>,
    ) -> crate::Result<Option<Candidate>> {
        let mut wallets: Vec<WalletDraw> = Vec::new();

        for candidate in candidates {
            let owner = self
                .wallets
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
        let Some(choice) = pick_by_power(&mut self.world.world_mut().rng, &powers) else {
            return Ok(None);
        };
        let mut machines = wallets.swap_remove(choice).machines;
        let machine = pick_machine(&mut self.world.world_mut().rng, machines.len());

        Ok(Some(machines.swap_remove(machine)))
    }

    async fn coin_jobs(&self, coin: CoinId) -> crate::Result<Vec<WorkId>> {
        Ok(self
            .mine_jobs
            .coin_subscribers(coin)
            .await?
            .iter()
            .map(|job| job.id)
            .collect())
    }
}

/// Mining-round candidate with sub-wallet and computer mining power.
struct Candidate {
    job: WorkId,
    subwallet: String,
    power: f64,
}

struct WalletDraw {
    owner: String,
    machines: Vec<Candidate>,
    power: f64,
}

impl Routine for CoinRound {
    type Data = CoinId;

    const KEY: &'static str = "economy.coin_round";
    const PRIORITY: i64 = 1;

    fn tables() -> TableSet {
        schema::COIN_ROUNDS
    }

    async fn store(&self, conn: &Connection, id: RoutineId, data: &CoinId) -> RoutineResult<()> {
        execute(
            conn,
            "INSERT INTO routine_mining_rounds (routine_id, coin_id) VALUES (?1, ?2)",
            params![id.cast_signed(), data.0],
        )
        .await?;

        Ok(())
    }

    async fn load(&self, conn: &Connection, id: RoutineId) -> RoutineResult<Option<CoinId>> {
        let coin = fetch_value::<u32>(
            conn,
            "SELECT coin_id FROM routine_mining_rounds WHERE routine_id = ?1",
            params![id.cast_signed()],
        )
        .await?;

        Ok(coin.map(CoinId))
    }

    async fn run(&self, _id: RoutineId, data: CoinId) -> RoutineResult<()> {
        self.execute(data).await.map_err(RoutineError::new)
    }
}
