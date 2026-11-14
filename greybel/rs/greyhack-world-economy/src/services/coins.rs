//! Coins: their records, passwords, creators, and mining rewards (`BlockchainSystem`).

use greyhack_behaviour::{cap_balance, round_reward};

use crate::{Coin, CoinId, MineJobFailure, PriceRecord, Result};

/// Methods of [`World`] for economy coins.
impl CoinService {
    pub async fn coins(&self) -> Result<Vec<Coin>> {
        self.coins.coins().await
    }

    pub async fn coin_id(&self, name: &str) -> Result<Option<CoinId>> {
        Ok(self.coins.coin_by_name(name).await?.map(|coin| coin.id))
    }

    pub async fn coin(&self, id: CoinId) -> Result<Option<Coin>> {
        self.coins.coin(id).await
    }

    pub async fn coin_by_name(&self, name: &str) -> Result<Option<Coin>> {
        self.coins.coin_by_name(name).await
    }

    pub async fn save_coin(&self, coin: &Coin) -> Result<()> {
        self.coins.save_coin(coin).await
    }

    pub async fn set_coin_password(&self, coin: CoinId, password: &str) -> Result<()> {
        let Some(mut row) = self.coins.coin(coin).await? else {
            return Ok(());
        };

        row.creator_password_hash = self.computers.register_password(password).await?;

        self.coins.save_coin(&row).await
    }

    pub async fn coins_of_creator(&self, creator_user: &str) -> Result<Vec<Coin>> {
        self.coins.coins_of_creator(creator_user).await
    }

    pub async fn mine(&self, coin: CoinId, user: &str) -> Result<Result<(), MineJobFailure>> {
        let Some(mut data) = self.coins.coin(coin).await? else {
            return Ok(Err(MineJobFailure::CoinGone));
        };

        if data.mined_coins >= data.max_supply {
            return Ok(Err(MineJobFailure::AllMined(data.name)));
        }

        let Some(mut subwallet) = self.subwallets.subwallet(coin, user).await? else {
            return Ok(Err(MineJobFailure::SubwalletGone));
        };
        let reward = round_reward(data.reward, data.max_supply, data.mined_coins);

        subwallet.balance = cap_balance(subwallet.balance + reward);
        data.mined_coins += reward;
        self.subwallets.save_subwallet(&subwallet).await?;
        if let Some(wallet) = self.wallets.wallet_by_user(&subwallet.wallet_user).await? {
            self.wallets
                .add_balance(wallet.id, &data.name, reward)
                .await?;
        }

        self.coins.save_coin(&data).await?;

        Ok(Ok(()))
    }
}

/// A world service with only its declared dependencies.
pub struct CoinService {
    pub(crate) world: greyhack_world_core::WorldRef,
    pub(crate) coins: std::rc::Rc<crate::storage::CoinRepository>,
    pub(crate) subwallets: std::rc::Rc<crate::storage::SubwalletRepository>,
    pub(crate) computers: greyhack_world_core::Dep<greyhack_world_core::ComputerService>,
    pub(crate) networks: greyhack_world_core::Dep<greyhack_world_core::NetworkService>,
    pub(crate) wallets: greyhack_world_core::Dep<crate::services::WalletService>,
    pub(crate) network_generation:
        greyhack_world_core::Dep<dyn greyhack_world_core::NetworkGeneration>,
    pub(crate) mine_jobs: greyhack_world_core::Dep<crate::routines::MineJobService>,
}

pub(crate) struct CoinServiceSupport {
    pub(crate) coins: std::rc::Rc<crate::storage::CoinRepository>,
    pub(crate) subwallets: std::rc::Rc<crate::storage::SubwalletRepository>,
    pub(crate) computers: greyhack_world_core::Dep<greyhack_world_core::ComputerService>,
    pub(crate) networks: greyhack_world_core::Dep<greyhack_world_core::NetworkService>,
    pub(crate) wallets: greyhack_world_core::Dep<crate::services::WalletService>,
    pub(crate) network_generation:
        greyhack_world_core::Dep<dyn greyhack_world_core::NetworkGeneration>,
    pub(crate) mine_jobs: greyhack_world_core::Dep<crate::routines::MineJobService>,
}

impl CoinService {
    pub(crate) fn new(
        world: &greyhack_world_core::WorldRef,
        dependencies: CoinServiceSupport,
    ) -> Self {
        Self {
            world: world.clone(),
            coins: dependencies.coins,
            subwallets: dependencies.subwallets,
            computers: dependencies.computers,
            networks: dependencies.networks,
            wallets: dependencies.wallets,
            network_generation: dependencies.network_generation,
            mine_jobs: dependencies.mine_jobs,
        }
    }
}

impl CoinService {
    /// Stores `coin` and registers its creator password when one is supplied.
    pub async fn insert_coin(&self, coin: &Coin, password: Option<&str>) -> Result<CoinId> {
        self.coins.insert_coin(coin, password).await
    }
}

impl CoinService {
    /// Returns the coin price records in chronological order.
    pub async fn price_history(&self, coin: CoinId) -> Result<Vec<PriceRecord>> {
        self.coins.price_history(coin).await
    }
}

impl CoinService {
    /// Appends `record` to the coin price history.
    pub async fn insert_price(&self, coin: CoinId, record: &PriceRecord) -> Result<()> {
        self.coins.insert_price(coin, record).await
    }
}

impl CoinService {
    /// Adds a matched trade to the pending daily price calculation.
    pub async fn record_trade(&self, coin: CoinId, price: i64, amount: i64) -> Result<()> {
        self.coins.record_trade(coin, price, amount).await
    }
}
