//! Wallets and per-coin balances, and what scripts do to them through `WalletIntrinsics`.

use greyhack_behaviour::{
    LOWERCASE_ALPHANUMERIC, SECONDS_PER_DAY, check_new_wallet, check_wallet_password, hash_password,
};

use crate::model::{Wallet, WalletId};
use crate::systems::economy::failure::EconomyFailure;
use crate::{ComputerId, Result, World};

/// Characters in a wallet PIN (`BlockchainSystem.Wallet.GetPin`).
const WALLET_PIN_LENGTH: u32 = 5;

impl World {
    /// Generates a five-character lowercase alphanumeric wallet PIN
    /// (`BlockchainSystem.Wallet.GetPin`).
    pub fn wallet_pin(&mut self) -> String {
        self.rng.text(LOWERCASE_ALPHANUMERIC, WALLET_PIN_LENGTH)
    }

    /// Returns wallet `id`.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn wallet(&self, id: WalletId) -> Result<Option<Wallet>> {
        self.db.wallet(id).await
    }

    /// Updates the `wallet` row.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn save_wallet(&mut self, wallet: &Wallet) -> Result<()> {
        self.db.save_wallet(wallet).await
    }

    /// Changes the `wallet` password.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn set_wallet_password(&mut self, wallet: WalletId, password: &str) -> Result<()> {
        let Some(mut row) = self.db.wallet(wallet).await? else {
            return Ok(());
        };

        row.password_hash = self.db.register_password(password).await?;

        self.db.save_wallet(&row).await
    }

    /// Returns the wallet owned by `user`.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn wallet_by_user(&self, user: &str) -> Result<Option<Wallet>> {
        self.db.wallet_by_user(user).await
    }

    /// Returns the wallet using bank account `number`.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn wallet_on_bank(&self, number: &str) -> Result<Option<Wallet>> {
        self.db.wallet_by_bank(number).await
    }

    /// Returns the wallet opened by `user` with `password`.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn login_wallet(&self, user: &str, password: &str) -> Result<Option<WalletId>> {
        let hash = hash_password(password);

        Ok(self
            .db
            .wallet_by_user(user)
            .await?
            .filter(|wallet| wallet.password_hash == hash)
            .map(|wallet| wallet.id))
    }

    /// Creates a wallet for `user` with `password`, `pin`, and `bank`; returns its ID.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn add_wallet(
        &mut self,
        user: &str,
        password: &str,
        pin: &str,
        bank: &str,
    ) -> Result<WalletId> {
        self.db
            .insert_wallet(
                &Wallet {
                    id: WalletId(0),
                    user: user.to_owned(),
                    password_hash: hash_password(password),
                    pin: pin.to_owned(),
                    bank: bank.to_owned(),
                    next_password_reset_at: None,
                },
                Some(password),
            )
            .await
    }

    /// Returns `wallet` balances as (coin name, amount), ordered by first acquisition.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn balances(&self, wallet: WalletId) -> Result<Vec<(String, f64)>> {
        self.db.balances(wallet).await
    }

    /// Returns `wallet`'s balance in coin `coin` if ever acquired.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn balance(&self, wallet: WalletId, coin: &str) -> Result<Option<f64>> {
        let Some(coin) = self.db.coin_by_name(coin).await? else {
            return Ok(None);
        };

        self.db.balance(wallet, coin.id).await
    }

    /// Sets `wallet` balance in coin `coin`. Returns whether wallet and coin exist.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn set_balance(&mut self, wallet: WalletId, coin: &str, amount: f64) -> Result<bool> {
        let (Some(_), Some(coin)) = (
            self.db.wallet(wallet).await?,
            self.db.coin_by_name(coin).await?,
        ) else {
            return Ok(false);
        };

        self.db.set_balance(wallet, coin.id, amount).await?;

        Ok(true)
    }

    /// Adds `delta` to `wallet` balance in coin `coin`; initial balances cannot become negative.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub(crate) async fn add_balance(
        &mut self,
        wallet: WalletId,
        coin: &str,
        delta: f64,
    ) -> Result<()> {
        let Some(coin) = self.db.coin_by_name(coin).await? else {
            return Ok(());
        };
        let amount = match self.db.balance(wallet, coin.id).await? {
            Some(amount) => amount + delta,
            None => delta.max(0.0),
        };

        self.db.set_balance(wallet, coin.id, amount).await
    }

    /// Returns the number of the bank account of the person on `computer`.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn bank_number_on(&self, computer: ComputerId) -> Result<Option<String>> {
        match self.person_on(computer).await? {
            Some(person) => self.bank_number_of(person).await,
            None => Ok(None),
        }
    }

    /// Creates a wallet for `user` and `password`, charging the bank account of the person on `host`
    /// (`BlockchainIntrinsics.create_wallet`).
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn create_wallet_for(
        &mut self,
        host: ComputerId,
        user: &str,
        password: &str,
    ) -> Result<std::result::Result<WalletId, EconomyFailure>> {
        if let Err(refusal) = check_new_wallet(user, password) {
            return Ok(Err(EconomyFailure::Refusal(refusal)));
        }

        let Some(bank) = self.bank_number_on(host).await? else {
            return Ok(Err(EconomyFailure::NoBankAccount));
        };

        if self.wallet_by_user(user).await?.is_some() {
            return Ok(Err(EconomyFailure::WalletUserExists));
        }

        if self.wallet_on_bank(&bank).await?.is_some() {
            return Ok(Err(EconomyFailure::OneWalletPerPlayer));
        }

        let pin = self.wallet_pin();
        let id = self.add_wallet(user, password, &pin, &bank).await?;
        let next_reset = self.time_seconds().saturating_add(SECONDS_PER_DAY);

        if let Some(mut wallet) = self.wallet(id).await? {
            wallet.next_password_reset_at = Some(next_reset);
            self.save_wallet(&wallet).await?;
        }

        Ok(Ok(id))
    }

    /// Sums the balances of all coins of `wallet` (`WalletIntrinsics.get_balance`).
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn total_balance(&self, wallet: WalletId) -> Result<f64> {
        Ok(self
            .balances(wallet)
            .await?
            .into_iter()
            .map(|(_, balance)| balance)
            .sum())
    }

    /// Changes the password of `wallet` after checking that the person on `host` owns it and that
    /// the password was not reset within a day (`WalletIntrinsics.reset_password`).
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn reset_wallet_password(
        &mut self,
        wallet: WalletId,
        host: ComputerId,
        password: &str,
    ) -> Result<std::result::Result<(), EconomyFailure>> {
        if let Err(refusal) = check_wallet_password(password) {
            return Ok(Err(EconomyFailure::Refusal(refusal)));
        }

        let now = self.time_seconds();
        let Some(mut wallet) = self.wallet(wallet).await? else {
            return Ok(Err(EconomyFailure::UnknownWallet));
        };
        let owner_bank = self.bank_number_on(host).await?;

        if owner_bank.as_deref() != Some(wallet.bank.as_str()) {
            return Ok(Err(EconomyFailure::NotOwner));
        }

        if wallet.next_password_reset_at.is_some_and(|at| now <= at) {
            return Ok(Err(EconomyFailure::PasswordResetTooSoon));
        }

        wallet.next_password_reset_at = Some(now.saturating_add(SECONDS_PER_DAY));
        self.save_wallet(&wallet).await?;
        self.set_wallet_password(wallet.id, password).await?;

        Ok(Ok(()))
    }
}
