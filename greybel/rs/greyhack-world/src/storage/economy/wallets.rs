//! Storage for wallets and their balances.

use turso::params;

use super::super::Database;
use super::super::sql::{FromRow, RowView, execute, fetch_all, fetch_opt, fetch_value, insert};
use crate::{CoinId, Result, Wallet, WalletId};

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

impl Database {
    /// Returns wallet `id`.
    pub(crate) async fn wallet(&self, id: WalletId) -> Result<Option<Wallet>> {
        Ok(fetch_opt(
            &self.conn,
            concat!("SELECT ", wallet_columns!(), " FROM wallets WHERE id = ?1"),
            params![id.0],
        )
        .await?)
    }

    /// Returns the wallet owned by `user`.
    pub(crate) async fn wallet_by_user(&self, user: &str) -> Result<Option<Wallet>> {
        Ok(fetch_opt(
            &self.conn,
            concat!(
                "SELECT ",
                wallet_columns!(),
                " FROM wallets WHERE user = ?1"
            ),
            params![user],
        )
        .await?)
    }

    /// Returns the wallet using bank account `number`.
    pub(crate) async fn wallet_by_bank(&self, number: &str) -> Result<Option<Wallet>> {
        Ok(fetch_opt(
            &self.conn,
            concat!(
                "SELECT ",
                wallet_columns!(),
                " FROM wallets WHERE bank_number = ?1"
            ),
            params![number],
        )
        .await?)
    }

    /// Inserts `wallet` with `password`, ignoring its ID, and returns the assigned ID.
    pub(crate) async fn insert_wallet(
        &self,
        wallet: &Wallet,
        password: Option<&str>,
    ) -> Result<WalletId> {
        if let Some(password) = password {
            self.register_password(password).await?;
        }

        let id = insert(
            &self.conn,
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

    /// Updates the `wallet` row.
    pub(crate) async fn save_wallet(&self, wallet: &Wallet) -> Result<()> {
        execute(
            &self.conn,
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

    /// Returns `wallet` balances as (coin name, amount), ordered by first acquisition.
    pub(crate) async fn balances(&self, wallet: WalletId) -> Result<Vec<(String, f64)>> {
        let rows: Vec<BalanceRow> = fetch_all(
            &self.conn,
            "SELECT coins.name, wallet_balances.amount FROM wallet_balances \
             JOIN coins ON coins.id = wallet_balances.coin_id WHERE wallet_balances.wallet_id = ?1 \
             ORDER BY wallet_balances.id",
            params![wallet.0],
        )
        .await?;

        Ok(rows.into_iter().map(|row| (row.name, row.amount)).collect())
    }

    /// Returns `wallet`'s balance in `coin` if it has ever been acquired.
    pub(crate) async fn balance(&self, wallet: WalletId, coin: CoinId) -> Result<Option<f64>> {
        Ok(fetch_value(
            &self.conn,
            "SELECT amount FROM wallet_balances WHERE wallet_id = ?1 AND coin_id = ?2",
            params![wallet.0, coin.0],
        )
        .await?)
    }

    /// Updates `wallet`'s balance in `coin` to `amount`.
    pub(crate) async fn set_balance(
        &self,
        wallet: WalletId,
        coin: CoinId,
        amount: f64,
    ) -> Result<()> {
        let rows = execute(
            &self.conn,
            "UPDATE wallet_balances SET amount = ?3 WHERE wallet_id = ?1 AND coin_id = ?2",
            params![wallet.0, coin.0, amount],
        )
        .await?;

        if rows == 0 {
            execute(
                &self.conn,
                "INSERT INTO wallet_balances (wallet_id, coin_id, amount) VALUES (?1, ?2, ?3)",
                params![wallet.0, coin.0, amount],
            )
            .await?;
        }

        Ok(())
    }
}
