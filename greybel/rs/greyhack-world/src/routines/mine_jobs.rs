//! Mine jobs run on a computer for a sub-wallet until winning a coin round.

use greyhack_behaviour::{GpuFault, check_gpu};

use crate::world::{MineJob, WorkId};
use crate::{CoinId, ComputerId, MineJobFailure, Result, WalletId, World};

impl World {
    /// Returns all mining programs in subscription order.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn mine_jobs(&self) -> Result<Vec<MineJob>> {
        self.db.mine_subscribers().await
    }

    /// Registers one sub-wallet mine job on the player computer using [`World::schedule_mine_on`].
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn schedule_mine(
        &mut self,
        coin: CoinId,
        subwallet: &str,
    ) -> Result<Result<WorkId, MineJobFailure>> {
        self.schedule_mine_on(coin, subwallet, self.player).await
    }

    /// Registers a sub-wallet mine job on `computer` and returns its ID or startup error. The job
    /// joins the next coin round immediately if the mining cycle has already elapsed.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn schedule_mine_on(
        &mut self,
        coin: CoinId,
        subwallet: &str,
        computer: ComputerId,
    ) -> Result<Result<WorkId, MineJobFailure>> {
        if !self.db.has_computer(computer).await? {
            return Ok(Err(MineJobFailure::ComputerUnavailable));
        }

        let Some(coin_data) = self.db.coin(coin).await? else {
            return Ok(Err(MineJobFailure::CoinGone));
        };

        if coin_data.mined_coins >= coin_data.max_supply {
            return Ok(Err(MineJobFailure::AllMined(coin_data.name)));
        }

        if self.db.subwallet(coin, subwallet).await?.is_none() {
            return Ok(Err(MineJobFailure::SubwalletGone));
        }

        if self
            .db
            .coin_subscribers(coin)
            .await?
            .iter()
            .any(|job| job.subwallet == subwallet && job.computer == computer)
        {
            return Ok(Err(MineJobFailure::SecondMineJob));
        }

        // The no-GPU CPU-decipher path of `SubWalletIntrinsics.mining` is not modeled.
        let hardware = self.db.hardware(computer).await?;

        if let Some(gpu) = hardware.gpus.first() {
            match check_gpu(&mut self.rng, gpu.part.health) {
                Some(GpuFault::LowHealth) => return Ok(Err(MineJobFailure::DamagedGpu)),
                Some(GpuFault::SelfTest) => return Ok(Err(MineJobFailure::CorruptedGpu)),
                None => {}
            }
        }

        let Some(id) = self
            .db
            .insert_mine_subscriber(coin, subwallet, computer)
            .await?
        else {
            return Ok(Err(MineJobFailure::SubwalletGone));
        };

        self.schedule_coin_round(coin).await?;

        Ok(Ok(id))
    }

    /// Counts pending wallet mine jobs for `coin`, as displayed by `show_nodes`.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn count_active_nodes(&self, wallet: WalletId, coin: CoinId) -> Result<usize> {
        let Some(wallet) = self.db.wallet(wallet).await? else {
            return Ok(0);
        };
        let mut count = 0;

        for job in self.db.coin_subscribers(coin).await? {
            if self
                .db
                .subwallet(coin, &job.subwallet)
                .await?
                .is_some_and(|subwallet| subwallet.wallet_user == wallet.user)
            {
                count += 1;
            }
        }

        Ok(count)
    }

    /// Deletes a coin as its creator, including balances, round, and sub-wallets, and fails its
    /// mine jobs.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn delete_coin(&mut self, name: &str) -> Result<bool> {
        let Some(coin) = self.db.coin_by_name(name).await? else {
            return Ok(false);
        };
        let mut work = self.coin_jobs(coin.id).await?;

        work.extend(self.db.coin_round(coin.id).await?);
        self.fail_work(&work, &MineJobFailure::CoinGone).await?;
        self.db.remove_coin(coin.id).await?;

        Ok(true)
    }

    /// Deletes coin sub-wallet `user` and fails its mine jobs.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn delete_subwallet(&mut self, coin: CoinId, user: &str) -> Result<bool> {
        let jobs: Vec<WorkId> = self
            .db
            .coin_subscribers(coin)
            .await?
            .iter()
            .filter(|job| job.subwallet == user)
            .map(|job| job.id)
            .collect();

        self.fail_work(&jobs, &MineJobFailure::SubwalletGone)
            .await?;

        self.remove_subwallet(coin, user).await
    }

    /// Cancels mine jobs on `computers` when their network is removed.
    pub(crate) async fn cancel_jobs_on(&mut self, computers: &[ComputerId]) -> Result<()> {
        let mut jobs = Vec::new();

        for &computer in computers {
            jobs.extend(
                self.db
                    .mine_subscribers_on(computer)
                    .await?
                    .iter()
                    .map(|job| job.id),
            );
        }

        self.cancel_work(&jobs).await
    }

    /// Returns IDs of jobs mining `coin`.
    pub(super) async fn coin_jobs(&self, coin: CoinId) -> Result<Vec<WorkId>> {
        Ok(self
            .db
            .coin_subscribers(coin)
            .await?
            .iter()
            .map(|job| job.id)
            .collect())
    }
}
