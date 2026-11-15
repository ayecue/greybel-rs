//! Mine jobs run on a computer for a sub-wallet until winning a coin round.

use std::rc::Rc;

use greyhack_behaviour::{GpuFault, check_gpu};
use greyhack_world_core::{CoinId, ComputerId, ComputerService, Result, WorldRef};
use greyhack_world_effects::WorkId;

use crate::storage::{MineJobRepository, RoutineRepository};
use crate::{
    CoinRound, CoinService, MineJob, MineJobFailure, MineJobState, WalletId, WalletService,
};

/// Mining subscriptions and their typed work results.
pub struct MineJobService {
    world: WorldRef,
    computers: greyhack_world_modules::Dep<ComputerService>,
    coins: greyhack_world_modules::Dep<CoinService>,
    wallets: greyhack_world_modules::Dep<WalletService>,
    jobs: Rc<MineJobRepository>,
    routines: Rc<RoutineRepository>,
    work: Rc<greyhack_world_effects::WorkQueue>,
    coin_round: greyhack_world_modules::Dep<CoinRound>,
}

pub(crate) struct MineJobServiceSupport {
    pub(crate) computers: greyhack_world_modules::Dep<ComputerService>,
    pub(crate) coins: greyhack_world_modules::Dep<CoinService>,
    pub(crate) wallets: greyhack_world_modules::Dep<WalletService>,
    pub(crate) jobs: Rc<MineJobRepository>,
    pub(crate) routines: Rc<RoutineRepository>,
    pub(crate) work: Rc<greyhack_world_effects::WorkQueue>,
    pub(crate) coin_round: greyhack_world_modules::Dep<CoinRound>,
}

impl MineJobService {
    pub(crate) fn new(world: &WorldRef, dependencies: MineJobServiceSupport) -> Self {
        Self {
            world: world.clone(),
            computers: dependencies.computers,
            coins: dependencies.coins,
            wallets: dependencies.wallets,
            jobs: dependencies.jobs,
            routines: dependencies.routines,
            work: dependencies.work,
            coin_round: dependencies.coin_round,
        }
    }

    /// Lists active mining subscriptions.
    pub async fn mine_jobs(&self) -> Result<Vec<MineJob>> {
        self.jobs.mine_subscribers().await
    }

    pub async fn schedule_mine(
        &self,
        coin: CoinId,
        subwallet: &str,
    ) -> Result<Result<WorkId, MineJobFailure>> {
        self.schedule_mine_on(coin, subwallet, self.world.player())
            .await
    }

    pub async fn schedule_mine_on(
        &self,
        coin: CoinId,
        subwallet: &str,
        computer: ComputerId,
    ) -> Result<Result<WorkId, MineJobFailure>> {
        if !self.computers.has_computer(computer).await? {
            return Ok(Err(MineJobFailure::ComputerUnavailable));
        }

        let Some(coin_data) = self.coins.coin(coin).await? else {
            return Ok(Err(MineJobFailure::CoinMissing));
        };

        if coin_data.mined_coins >= coin_data.max_supply {
            return Ok(Err(MineJobFailure::AllMined(coin_data.name)));
        }

        if self.wallets.subwallet(coin, subwallet).await?.is_none() {
            return Ok(Err(MineJobFailure::SubwalletMissing));
        }

        if self
            .jobs
            .coin_subscribers(coin)
            .await?
            .iter()
            .any(|job| job.subwallet == subwallet && job.computer == computer)
        {
            return Ok(Err(MineJobFailure::SecondMineJob));
        }

        // The no-GPU CPU-decipher path of `SubWalletIntrinsics.mining` is not modeled.
        let hardware = self.computers.hardware(computer).await?;

        if let Some(gpu) = hardware.gpus.first() {
            match check_gpu(&mut self.world.world_mut().rng, gpu.part.health) {
                Some(GpuFault::LowHealth) => return Ok(Err(MineJobFailure::DamagedGpu)),
                Some(GpuFault::SelfTest) => return Ok(Err(MineJobFailure::CorruptedGpu)),
                None => {}
            }
        }

        let Some(id) = self
            .jobs
            .insert_mine_subscriber(coin, subwallet, computer)
            .await?
        else {
            return Ok(Err(MineJobFailure::SubwalletMissing));
        };

        self.work.link_computer(id, computer.0);
        self.coin_round.schedule(coin).await?;

        Ok(Ok(id))
    }

    pub async fn count_active_nodes(&self, wallet: WalletId, coin: CoinId) -> Result<usize> {
        let Some(wallet) = self.wallets.wallet(wallet).await? else {
            return Ok(0);
        };
        let mut count = 0;

        for job in self.jobs.coin_subscribers(coin).await? {
            if self
                .wallets
                .subwallet(coin, &job.subwallet)
                .await?
                .is_some_and(|subwallet| subwallet.wallet_user == wallet.user)
            {
                count += 1;
            }
        }

        Ok(count)
    }

    pub async fn delete_coin(&self, name: &str) -> Result<bool> {
        let Some(coin) = self.coins.coin_by_name(name).await? else {
            return Ok(false);
        };
        let mut work = self.coin_jobs(coin.id).await?;

        work.extend(self.routines.coin_round(coin.id).await?.map(WorkId::from));
        self.fail(&work, &MineJobFailure::CoinMissing).await?;
        self.coins.coins.remove_coin(coin.id).await?;

        Ok(true)
    }

    pub async fn delete_subwallet(&self, coin: CoinId, user: &str) -> Result<bool> {
        let jobs: Vec<WorkId> = self
            .jobs
            .coin_subscribers(coin)
            .await?
            .iter()
            .filter(|job| job.subwallet == user)
            .map(|job| job.id)
            .collect();

        self.fail(&jobs, &MineJobFailure::SubwalletMissing).await?;

        self.wallets.remove_subwallet(coin, user).await
    }

    pub(crate) async fn end<T: 'static>(&self, id: WorkId, result: T) -> Result<()> {
        if self.jobs.remove_mine_subscriber(id).await? {
            self.work.finish(id, result);
        }

        Ok(())
    }

    pub(crate) async fn fail(&self, ids: &[WorkId], failure: &MineJobFailure) -> Result<()> {
        for &id in ids {
            self.end(id, Err::<(), _>(failure.clone())).await?;
        }

        Ok(())
    }

    /// Cancels mine job `id` and reports whether it existed.
    pub async fn cancel(&self, id: WorkId) -> Result<bool> {
        let present = self.jobs.remove_mine_subscriber(id).await?;

        if present {
            self.work.cancel(id);
        }

        Ok(present)
    }

    pub async fn cancel_jobs_on(&self, computers: &[ComputerId]) -> Result<()> {
        let mut jobs = Vec::new();

        for &computer in computers {
            jobs.extend(
                self.jobs
                    .mine_subscribers_on(computer)
                    .await?
                    .iter()
                    .map(|job| job.id),
            );
        }

        for id in jobs {
            self.cancel(id).await?;
        }

        Ok(())
    }

    pub(crate) async fn coin_subscribers(&self, coin: CoinId) -> Result<Vec<MineJob>> {
        self.jobs.coin_subscribers(coin).await
    }

    pub async fn coin_jobs(&self, coin: CoinId) -> Result<Vec<WorkId>> {
        Ok(self
            .jobs
            .coin_subscribers(coin)
            .await?
            .iter()
            .map(|job| job.id)
            .collect())
    }
}

impl greyhack_world_effects::WorkOwner for MineJobService {
    fn cancel(&self, id: WorkId) -> greyhack_world_effects::WorkCancelFuture<'_> {
        Box::pin(async move {
            MineJobService::cancel(self, id)
                .await
                .map_err(greyhack_world_effects::EventError::new)
        })
    }

    fn clear_tree(&self, id: WorkId, state: &mut greyhack_world_programs::StateMap) {
        let job = state.state_mut::<MineJobState>();

        if job.work == Some(id) {
            job.work = None;
        }
    }
}
