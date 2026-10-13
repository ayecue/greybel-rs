//! Coins: their records, passwords, creators, and mining rewards (`BlockchainSystem`).

use greyhack_behaviour::{cap_balance, round_reward};

use crate::{Coin, CoinId, MineJobFailure, Result, World};

impl World {
    /// Returns all coins in creation order.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn coins(&self) -> Result<Vec<Coin>> {
        self.db.coins().await
    }

    /// Returns the ID of coin `name`.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn coin_id(&self, name: &str) -> Result<Option<CoinId>> {
        Ok(self.db.coin_by_name(name).await?.map(|coin| coin.id))
    }

    /// Returns coin `id`.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn coin(&self, id: CoinId) -> Result<Option<Coin>> {
        self.db.coin(id).await
    }

    /// Returns the coin named `name`.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn coin_by_name(&self, name: &str) -> Result<Option<Coin>> {
        self.db.coin_by_name(name).await
    }

    /// Updates the `coin` row.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn save_coin(&mut self, coin: &Coin) -> Result<()> {
        self.db.save_coin(coin).await
    }

    /// Changes the creator password for `coin` and registers its hash
    /// (`BlockchainSystem.ResetCoinPass`).
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn set_coin_password(&mut self, coin: CoinId, password: &str) -> Result<()> {
        let Some(mut row) = self.db.coin(coin).await? else {
            return Ok(());
        };

        row.creator_password_hash = self.db.register_password(password).await?;

        self.db.save_coin(&row).await
    }

    /// Returns coins created by `creator_user` in creation order.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn coins_of_creator(&self, creator_user: &str) -> Result<Vec<Coin>> {
        self.db.coins_of_creator(creator_user).await
    }

    /// Pays a mining reward to `coin` sub-wallet `user`, limited by remaining coin supply.
    ///
    /// # Errors
    ///
    /// Returns any database error; the inner error explains why no reward can be paid.
    pub async fn mine(&mut self, coin: CoinId, user: &str) -> Result<Result<(), MineJobFailure>> {
        let Some(mut data) = self.db.coin(coin).await? else {
            return Ok(Err(MineJobFailure::CoinGone));
        };

        if data.mined_coins >= data.max_supply {
            return Ok(Err(MineJobFailure::AllMined(data.name)));
        }

        let Some(mut subwallet) = self.db.subwallet(coin, user).await? else {
            return Ok(Err(MineJobFailure::SubwalletGone));
        };
        let reward = round_reward(data.reward, data.max_supply, data.mined_coins);

        subwallet.balance = cap_balance(subwallet.balance + reward);
        data.mined_coins += reward;
        self.db.save_subwallet(&subwallet).await?;
        if let Some(wallet) = self.db.wallet_by_user(&subwallet.wallet_user).await? {
            self.add_balance(wallet.id, &data.name, reward).await?;
        }

        self.db.save_coin(&data).await?;

        Ok(Ok(()))
    }
}
