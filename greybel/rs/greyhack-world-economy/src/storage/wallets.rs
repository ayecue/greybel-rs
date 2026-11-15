//! Storage for wallets and their balances.

use greyhack_world_core::{CoinId, Database, Result};
use greyhack_world_storage::sql::{
    FromRow, RowView, execute, fetch_all, fetch_opt, fetch_value, insert,
};
use turso::params;

use crate::{Wallet, WalletId};

/// Wallet columns in [`Wallet`] [`FromRow`] order.
macro_rules! wallet_columns {
    () => {
        "id, user, password_hash, pin, bank_number, password_reset_at"
    };
}

impl FromRow for Wallet {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            id: WalletId(row.get("id")?),
            user: row.get("user")?,
            password_hash: row.get("password_hash")?,
            pin: row.get("pin")?,
            bank: row.get("bank_number")?,
            next_password_reset_at: row.get("password_reset_at")?,
        })
    }
}

/// Wallet balance for one coin, keyed by coin name.
struct BalanceRow {
    name: String,
    amount: f64,
}

impl FromRow for BalanceRow {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            name: row.get("name")?,
            amount: row.get("amount")?,
        })
    }
}

/// Queries of [`Database`] for economy wallets.
/// Queries of the tables owned by this area.
pub(crate) struct WalletRepository {
    db: Database,
    computers: greyhack_world_modules::Dep<greyhack_world_core::ComputerService>,
}

impl WalletRepository {
    pub(crate) fn new(
        db: Database,
        computers: greyhack_world_modules::Dep<greyhack_world_core::ComputerService>,
    ) -> Self {
        Self { db, computers }
    }

    pub(crate) async fn wallet(&self, id: WalletId) -> Result<Option<Wallet>> {
        Ok(fetch_opt(
            &self.db.connection(),
            concat!("SELECT ", wallet_columns!(), " FROM wallets WHERE id = ?1"),
            params![id.0],
        )
        .await?)
    }

    pub(crate) async fn wallet_by_user(&self, user: &str) -> Result<Option<Wallet>> {
        Ok(fetch_opt(
            &self.db.connection(),
            concat!(
                "SELECT ",
                wallet_columns!(),
                " FROM wallets WHERE user = ?1"
            ),
            params![user],
        )
        .await?)
    }

    pub(crate) async fn wallet_by_bank(&self, number: &str) -> Result<Option<Wallet>> {
        Ok(fetch_opt(
            &self.db.connection(),
            concat!(
                "SELECT ",
                wallet_columns!(),
                " FROM wallets WHERE bank_number = ?1"
            ),
            params![number],
        )
        .await?)
    }

    pub(crate) async fn insert_wallet(
        &self,
        wallet: &Wallet,
        password: Option<&str>,
    ) -> Result<WalletId> {
        if let Some(password) = password {
            self.computers.register_password(password).await?;
        }

        let id = insert(
            &self.db.connection(),
            "INSERT INTO wallets (user, password_hash, pin, bank_number, password_reset_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                wallet.user.as_str(),
                wallet.password_hash.as_str(),
                wallet.pin.as_str(),
                wallet.bank.as_str(),
                wallet.next_password_reset_at.map(u64::cast_signed),
            ],
        )
        .await?;

        Ok(WalletId(u32::try_from(id).expect("wallet ids fit u32")))
    }

    pub(crate) async fn save_wallet(&self, wallet: &Wallet) -> Result<()> {
        execute(
            &self.db.connection(),
            "UPDATE wallets SET user = ?2, password_hash = ?3, pin = ?4, bank_number = ?5, password_reset_at = ?6 \
             WHERE id = ?1",
            params![
                wallet.id.0,
                wallet.user.as_str(),
                wallet.password_hash.as_str(),
                wallet.pin.as_str(),
                wallet.bank.as_str(),
                wallet.next_password_reset_at.map(u64::cast_signed),
            ],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn balances(&self, wallet: WalletId) -> Result<Vec<(String, f64)>> {
        let rows: Vec<BalanceRow> = fetch_all(
            &self.db.connection(),
            "SELECT coins.name, wallet_balances.amount FROM wallet_balances \
             JOIN coins ON coins.id = wallet_balances.coin_id WHERE wallet_balances.wallet_id = ?1 \
             ORDER BY wallet_balances.id",
            params![wallet.0],
        )
        .await?;

        Ok(rows.into_iter().map(|row| (row.name, row.amount)).collect())
    }

    pub(crate) async fn balance(&self, wallet: WalletId, coin: CoinId) -> Result<Option<f64>> {
        Ok(fetch_value(
            &self.db.connection(),
            "SELECT amount FROM wallet_balances WHERE wallet_id = ?1 AND coin_id = ?2",
            params![wallet.0, coin.0],
        )
        .await?)
    }

    pub(crate) async fn set_balance(
        &self,
        wallet: WalletId,
        coin: CoinId,
        amount: f64,
    ) -> Result<()> {
        let rows = execute(
            &self.db.connection(),
            "UPDATE wallet_balances SET amount = ?3 WHERE wallet_id = ?1 AND coin_id = ?2",
            params![wallet.0, coin.0, amount],
        )
        .await?;

        if rows == 0 {
            execute(
                &self.db.connection(),
                "INSERT INTO wallet_balances (wallet_id, coin_id, amount) VALUES (?1, ?2, ?3)",
                params![wallet.0, coin.0, amount],
            )
            .await?;
        }

        Ok(())
    }
}
