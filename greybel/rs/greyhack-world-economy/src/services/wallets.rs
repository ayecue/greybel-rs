//! Wallets and per-coin balances, and what scripts do to them through `WalletIntrinsics`.

use greyhack_behaviour::{
    LOWERCASE_ALPHANUMERIC, SECONDS_PER_DAY, check_new_wallet, check_wallet_password, hash_password,
};
use greyhack_world_core::{ComputerId, Result};

use crate::model::{Wallet, WalletId};
use crate::services::failure::EconomyFailure;

/// Characters in a wallet PIN (`BlockchainSystem.Wallet.GetPin`).
const WALLET_PIN_LENGTH: u32 = 5;

/// Methods of [`World`] for economy wallets.
impl WalletService {
    pub fn wallet_pin(&self) -> String {
        self.world
            .world_mut()
            .rng
            .text(LOWERCASE_ALPHANUMERIC, WALLET_PIN_LENGTH)
    }

    pub async fn wallet(&self, id: WalletId) -> Result<Option<Wallet>> {
        self.wallets.wallet(id).await
    }

    pub async fn save_wallet(&self, wallet: &Wallet) -> Result<()> {
        self.wallets.save_wallet(wallet).await
    }

    pub async fn set_wallet_password(&self, wallet: WalletId, password: &str) -> Result<()> {
        let Some(mut row) = self.wallets.wallet(wallet).await? else {
            return Ok(());
        };

        row.password_hash = self.computers.register_password(password).await?;

        self.wallets.save_wallet(&row).await
    }

    pub async fn wallet_by_user(&self, user: &str) -> Result<Option<Wallet>> {
        self.wallets.wallet_by_user(user).await
    }

    pub async fn wallet_on_bank(&self, number: &str) -> Result<Option<Wallet>> {
        self.wallets.wallet_by_bank(number).await
    }

    pub async fn login_wallet(&self, user: &str, password: &str) -> Result<Option<WalletId>> {
        let hash = hash_password(password);

        Ok(self
            .wallets
            .wallet_by_user(user)
            .await?
            .filter(|wallet| wallet.password_hash == hash)
            .map(|wallet| wallet.id))
    }

    pub async fn add_wallet(
        &self,
        user: &str,
        password: &str,
        pin: &str,
        bank: &str,
    ) -> Result<WalletId> {
        self.wallets
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

    pub async fn balances(&self, wallet: WalletId) -> Result<Vec<(String, f64)>> {
        self.wallets.balances(wallet).await
    }

    pub async fn balance(&self, wallet: WalletId, coin: &str) -> Result<Option<f64>> {
        let Some(coin) = self.coins.coin_by_name(coin).await? else {
            return Ok(None);
        };

        self.wallets.balance(wallet, coin.id).await
    }

    pub async fn set_balance(&self, wallet: WalletId, coin: &str, amount: f64) -> Result<bool> {
        let (Some(_), Some(coin)) = (
            self.wallets.wallet(wallet).await?,
            self.coins.coin_by_name(coin).await?,
        ) else {
            return Ok(false);
        };

        self.wallets.set_balance(wallet, coin.id, amount).await?;

        Ok(true)
    }

    pub async fn add_balance(&self, wallet: WalletId, coin: &str, delta: f64) -> Result<()> {
        let Some(coin) = self.coins.coin_by_name(coin).await? else {
            return Ok(());
        };
        let amount = match self.wallets.balance(wallet, coin.id).await? {
            Some(amount) => amount + delta,
            None => delta.max(0.0),
        };

        self.wallets.set_balance(wallet, coin.id, amount).await
    }

    pub async fn bank_number_on(&self, computer: ComputerId) -> Result<Option<String>> {
        match self.computers.person_on(computer).await? {
            Some(person) => self.bank.bank_number_of(person).await,
            None => Ok(None),
        }
    }

    pub async fn create_wallet_for(
        &self,
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
        let next_reset = self.world.time_seconds().saturating_add(SECONDS_PER_DAY);

        if let Some(mut wallet) = self.wallet(id).await? {
            wallet.next_password_reset_at = Some(next_reset);
            self.save_wallet(&wallet).await?;
        }

        Ok(Ok(id))
    }

    pub async fn total_balance(&self, wallet: WalletId) -> Result<f64> {
        Ok(self
            .balances(wallet)
            .await?
            .into_iter()
            .map(|(_, balance)| balance)
            .sum())
    }

    pub async fn reset_wallet_password(
        &self,
        wallet: WalletId,
        host: ComputerId,
        password: &str,
    ) -> Result<std::result::Result<(), EconomyFailure>> {
        if let Err(refusal) = check_wallet_password(password) {
            return Ok(Err(EconomyFailure::Refusal(refusal)));
        }

        let now = self.world.time_seconds();
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

/// A world service with only its declared dependencies.
pub struct WalletService {
    pub(crate) world: greyhack_world_core::WorldRef,
    pub(crate) wallets: std::rc::Rc<crate::storage::WalletRepository>,
    pub(crate) coins: std::rc::Rc<crate::storage::CoinRepository>,
    pub(crate) subwallets: std::rc::Rc<crate::storage::SubwalletRepository>,
    pub(crate) offers: std::rc::Rc<crate::storage::OfferRepository>,
    pub(crate) computers: greyhack_world_modules::Dep<greyhack_world_core::ComputerService>,
    pub(crate) bank: greyhack_world_modules::Dep<crate::services::Bank>,
}

impl WalletService {
    pub(crate) fn new(
        world: &greyhack_world_core::WorldRef,
        wallets: std::rc::Rc<crate::storage::WalletRepository>,
        coins: std::rc::Rc<crate::storage::CoinRepository>,
        subwallets: std::rc::Rc<crate::storage::SubwalletRepository>,
        offers: std::rc::Rc<crate::storage::OfferRepository>,
        computers: greyhack_world_modules::Dep<greyhack_world_core::ComputerService>,
        bank: greyhack_world_modules::Dep<crate::services::Bank>,
    ) -> Self {
        Self {
            world: world.clone(),
            wallets,
            coins,
            subwallets,
            offers,
            computers,
            bank,
        }
    }
}
