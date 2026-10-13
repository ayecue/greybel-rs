//! Coin sub-wallets and the transactions between them (`Coin.Transaction`).

use greyhack_behaviour::clamp_balance;

use crate::{CoinId, Result, Side, SubWallet, Transaction, TransferFailure, World};

impl World {
    /// Returns `coin` sub-wallet `user`.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn subwallet(&self, coin: CoinId, user: &str) -> Result<Option<SubWallet>> {
        self.db.subwallet(coin, user).await
    }

    /// Returns `coin` sub-wallets in creation order.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn subwallets(&self, coin: CoinId) -> Result<Vec<SubWallet>> {
        self.db.subwallets(coin).await
    }

    /// Updates the `subwallet` row.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn save_subwallet(&mut self, subwallet: &SubWallet) -> Result<()> {
        self.db.save_subwallet(subwallet).await
    }

    /// Creates `subwallet` using `password`.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn add_subwallet(&mut self, subwallet: &SubWallet, password: &str) -> Result<()> {
        self.db.insert_subwallet(subwallet, Some(password)).await?;

        if let Some(wallet) = self.db.wallet_by_user(&subwallet.wallet_user).await?
            && self.db.balance(wallet.id, subwallet.coin).await?.is_none()
        {
            self.db.set_balance(wallet.id, subwallet.coin, 0.0).await?;
        }

        Ok(())
    }

    /// Returns the last transfer for `coin` sub-wallet `user`.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn last_transaction(&self, coin: CoinId, user: &str) -> Result<Option<Transaction>> {
        self.db.last_transaction(coin, user).await
    }

    /// Deletes `coin` sub-wallet `user` and its wallet offers on the coin, subtracts its balance
    /// from mined supply, and returns whether it existed.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub(crate) async fn remove_subwallet(&mut self, coin: CoinId, user: &str) -> Result<bool> {
        let (Some(mut data), Some(subwallet)) = (
            self.db.coin(coin).await?,
            self.db.subwallet(coin, user).await?,
        ) else {
            return Ok(false);
        };

        if let Some(wallet) = self.db.wallet_by_user(&subwallet.wallet_user).await? {
            self.db.remove_offer(coin, wallet.id).await?;
            self.add_balance(wallet.id, &data.name, -subwallet.balance)
                .await?;
        }

        self.db.remove_subwallet(coin, user).await?;
        data.mined_coins = (data.mined_coins - subwallet.balance).max(0.0);
        self.db.save_coin(&data).await?;

        Ok(true)
    }

    /// Transfers `amount` of `coin` from sub-wallet `origin` to `destination` (`Coin.Transaction`)
    /// and records it as both sides' last transaction.
    ///
    /// # Errors
    ///
    /// Returns any database error; the inner error explains why the transaction was rejected.
    pub async fn transaction(
        &mut self,
        coin: CoinId,
        origin: &str,
        destination: &str,
        amount: i64,
        date: &str,
    ) -> Result<Result<(), TransferFailure>> {
        let Some(coin_data) = self.db.coin(coin).await? else {
            return Ok(Err(TransferFailure::UnknownCoin));
        };

        let Some(mut from) = self.db.subwallet(coin, origin).await? else {
            return Ok(Err(TransferFailure::MissingSubwallet(origin.to_owned())));
        };
        let Some(mut to) = self.db.subwallet(coin, destination).await? else {
            return Ok(Err(TransferFailure::MissingSubwallet(
                destination.to_owned(),
            )));
        };

        let amount = amount as f64;

        if from.balance < 0.0 {
            return Ok(Err(TransferFailure::NoCoins(origin.to_owned())));
        }

        if from.balance < amount {
            return Ok(Err(TransferFailure::InsufficientCoins));
        }

        // Reserve coins offered by pending sub-wallet sales (`BlockchainSystem.Transaction`).
        if origin != destination
            && let Some(wallet) = self.db.wallet_by_user(&from.wallet_user).await?
            && let Some(offer) = self.db.offer_of(coin, wallet.id).await?
            && offer.side == Side::Sell
            && offer.subwallet == origin
            && from.balance - amount < offer.amount
        {
            return Ok(Err(TransferFailure::SaleOfferShortfall));
        }

        // `Coin.Transaction` applies signed amounts without rejecting zero or negatives;
        // `SubWallet.AddAmount` clamps each resulting balance (`BlockchainSystem.AddAmount`).
        from.balance = clamp_balance(from.balance - amount);
        self.db.save_subwallet(&from).await?;
        if let Some(wallet) = self.db.wallet_by_user(&from.wallet_user).await? {
            self.add_balance(wallet.id, &coin_data.name, -amount)
                .await?;
        }

        self.db
            .set_last_transaction(
                coin,
                origin,
                &Transaction {
                    dest_account: destination.to_owned(),
                    amount,
                    direction: 1,
                    date: date.to_owned(),
                },
            )
            .await?;

        if destination == origin {
            to.balance = from.balance;
        }

        to.balance = clamp_balance(to.balance + amount);
        self.db.save_subwallet(&to).await?;
        if let Some(wallet) = self.db.wallet_by_user(&to.wallet_user).await? {
            self.add_balance(wallet.id, &coin_data.name, amount).await?;
        }

        self.db
            .set_last_transaction(
                coin,
                destination,
                &Transaction {
                    dest_account: origin.to_owned(),
                    amount,
                    direction: 0,
                    date: date.to_owned(),
                },
            )
            .await?;

        Ok(Ok(()))
    }
}

#[cfg(test)]
#[path = "tests/subwallets.rs"]
mod tests;
