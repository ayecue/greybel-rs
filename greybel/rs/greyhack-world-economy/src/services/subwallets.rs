//! Coin sub-wallets and the transactions between them (`Coin.Transaction`).

use greyhack_behaviour::{Side, clamp_balance};
use greyhack_world_core::{CoinId, Result};

use crate::services::WalletService;
use crate::{SubWallet, Transaction, TransferFailure};

/// Methods of [`World`] for economy subwallets.
impl WalletService {
    pub async fn subwallet(&self, coin: CoinId, user: &str) -> Result<Option<SubWallet>> {
        self.subwallets.subwallet(coin, user).await
    }

    pub async fn subwallets(&self, coin: CoinId) -> Result<Vec<SubWallet>> {
        self.subwallets.subwallets(coin).await
    }

    pub async fn save_subwallet(&self, subwallet: &SubWallet) -> Result<()> {
        self.subwallets.save_subwallet(subwallet).await
    }

    pub async fn add_subwallet(&self, subwallet: &SubWallet, password: &str) -> Result<()> {
        self.subwallets
            .insert_subwallet(subwallet, Some(password))
            .await?;

        if let Some(wallet) = self.wallets.wallet_by_user(&subwallet.wallet_user).await?
            && self
                .wallets
                .balance(wallet.id, subwallet.coin)
                .await?
                .is_none()
        {
            self.wallets
                .set_balance(wallet.id, subwallet.coin, 0.0)
                .await?;
        }

        Ok(())
    }

    pub async fn last_transaction(&self, coin: CoinId, user: &str) -> Result<Option<Transaction>> {
        self.subwallets.last_transaction(coin, user).await
    }

    pub async fn remove_subwallet(&self, coin: CoinId, user: &str) -> Result<bool> {
        let (Some(mut data), Some(subwallet)) = (
            self.coins.coin(coin).await?,
            self.subwallets.subwallet(coin, user).await?,
        ) else {
            return Ok(false);
        };

        if let Some(wallet) = self.wallets.wallet_by_user(&subwallet.wallet_user).await? {
            self.offers.remove_offer(coin, wallet.id).await?;
            self.add_balance(wallet.id, &data.name, -subwallet.balance)
                .await?;
        }

        self.subwallets.remove_subwallet(coin, user).await?;
        data.mined_coins = (data.mined_coins - subwallet.balance).max(0.0);
        self.coins.save_coin(&data).await?;

        Ok(true)
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "BlockchainSystem.Transaction converts its integral amount to a floating-point balance"
    )]
    pub async fn transaction(
        &self,
        coin: CoinId,
        origin: &str,
        destination: &str,
        amount: i64,
        date: &str,
    ) -> Result<Result<(), TransferFailure>> {
        let Some(coin_data) = self.coins.coin(coin).await? else {
            return Ok(Err(TransferFailure::UnknownCoin));
        };

        let Some(mut from) = self.subwallets.subwallet(coin, origin).await? else {
            return Ok(Err(TransferFailure::MissingSubwallet(origin.to_owned())));
        };
        let Some(mut to) = self.subwallets.subwallet(coin, destination).await? else {
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
            && let Some(wallet) = self.wallets.wallet_by_user(&from.wallet_user).await?
            && let Some(offer) = self.offers.offer_of(coin, wallet.id).await?
            && offer.side == Side::Sell
            && offer.subwallet == origin
            && from.balance - amount < offer.amount
        {
            return Ok(Err(TransferFailure::SaleOfferShortfall));
        }

        // `Coin.Transaction` applies signed amounts without rejecting zero or negatives;
        // `SubWallet.AddAmount` clamps each resulting balance (`BlockchainSystem.AddAmount`).
        from.balance = clamp_balance(from.balance - amount);
        self.subwallets.save_subwallet(&from).await?;
        if let Some(wallet) = self.wallets.wallet_by_user(&from.wallet_user).await? {
            self.add_balance(wallet.id, &coin_data.name, -amount)
                .await?;
        }

        self.subwallets
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
        self.subwallets.save_subwallet(&to).await?;
        if let Some(wallet) = self.wallets.wallet_by_user(&to.wallet_user).await? {
            self.add_balance(wallet.id, &coin_data.name, amount).await?;
        }

        self.subwallets
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
