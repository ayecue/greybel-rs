//! Storage for sub-wallets and their last transactions.

use greyhack_world_core::Database;
use greyhack_world_storage::sql::{FromRow, RowView, execute, fetch_all, fetch_opt};
use turso::params;

use crate::{CoinId, Result, SubWallet, Transaction};

impl FromRow for SubWallet {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            coin: CoinId(row.get("coin_id")?),
            user: row.get("user")?,
            password_hash: row.get("password_hash")?,
            balance: row.get("balance")?,
            info: row.get("info")?,
            wallet_user: row.get("wallet_user")?,
        })
    }
}

impl FromRow for Transaction {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            dest_account: row.get("counterparty")?,
            amount: row.get("amount")?,
            direction: row.get("direction")?,
            date: row.get("date")?,
        })
    }
}

/// Queries of [`Database`] for economy subwallets.
/// Queries of the tables owned by this area.
pub(crate) struct SubwalletRepository {
    db: Database,
    computers: greyhack_world_core::Dep<greyhack_world_core::ComputerService>,
}

impl SubwalletRepository {
    pub(crate) fn new(
        db: Database,
        computers: greyhack_world_core::Dep<greyhack_world_core::ComputerService>,
    ) -> Self {
        Self { db, computers }
    }

    pub(crate) async fn subwallets(&self, coin: CoinId) -> Result<Vec<SubWallet>> {
        Ok(fetch_all(
            &self.db.connection(),
            "SELECT subwallets.coin_id, subwallets.user, subwallets.password_hash, subwallets.balance, subwallets.info, \
             wallets.user AS wallet_user FROM subwallets JOIN wallets ON wallets.id = subwallets.wallet_id \
             WHERE subwallets.coin_id = ?1 ORDER BY subwallets.id",
            params![coin.0],
        )
        .await?)
    }

    pub(crate) async fn subwallet(&self, coin: CoinId, user: &str) -> Result<Option<SubWallet>> {
        Ok(fetch_opt(
            &self.db.connection(),
            "SELECT subwallets.coin_id, subwallets.user, subwallets.password_hash, subwallets.balance, subwallets.info, \
             wallets.user AS wallet_user FROM subwallets JOIN wallets ON wallets.id = subwallets.wallet_id \
             WHERE subwallets.coin_id = ?1 AND subwallets.user = ?2",
            params![coin.0, user],
        )
        .await?)
    }

    pub(crate) async fn insert_subwallet(
        &self,
        subwallet: &SubWallet,
        password: Option<&str>,
    ) -> Result<()> {
        if let Some(password) = password {
            self.computers.register_password(password).await?;
        }

        execute(
            &self.db.connection(),
            "INSERT INTO subwallets (coin_id, user, password_hash, balance, info, wallet_id) \
             VALUES (?1, ?2, ?3, ?4, ?5, (SELECT id FROM wallets WHERE user = ?6))",
            params![
                subwallet.coin.0,
                subwallet.user.as_str(),
                subwallet.password_hash.as_str(),
                subwallet.balance,
                subwallet.info.as_str(),
                subwallet.wallet_user.as_str(),
            ],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn save_subwallet(&self, subwallet: &SubWallet) -> Result<()> {
        execute(
            &self.db.connection(),
            "UPDATE subwallets SET password_hash = ?3, balance = ?4, info = ?5, \
             wallet_id = (SELECT id FROM wallets WHERE user = ?6) WHERE coin_id = ?1 AND user = ?2",
            params![
                subwallet.coin.0,
                subwallet.user.as_str(),
                subwallet.password_hash.as_str(),
                subwallet.balance,
                subwallet.info.as_str(),
                subwallet.wallet_user.as_str(),
            ],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn remove_subwallet(&self, coin: CoinId, user: &str) -> Result<bool> {
        Ok(execute(
            &self.db.connection(),
            "DELETE FROM subwallets WHERE coin_id = ?1 AND user = ?2",
            params![coin.0, user],
        )
        .await?
            > 0)
    }

    pub(crate) async fn last_transaction(
        &self,
        coin: CoinId,
        user: &str,
    ) -> Result<Option<Transaction>> {
        Ok(fetch_opt(
            &self.db.connection(),
            "SELECT subwallet_transactions.counterparty, subwallet_transactions.amount, \
             subwallet_transactions.direction, subwallet_transactions.date FROM subwallet_transactions \
             JOIN subwallets ON subwallets.id = subwallet_transactions.subwallet_id \
             WHERE subwallets.coin_id = ?1 AND subwallets.user = ?2",
            params![coin.0, user],
        )
        .await?)
    }

    pub(crate) async fn set_last_transaction(
        &self,
        coin: CoinId,
        user: &str,
        transaction: &Transaction,
    ) -> Result<()> {
        execute(
            &self.db.connection(),
            "INSERT OR REPLACE INTO subwallet_transactions (subwallet_id, counterparty, amount, direction, date) \
             SELECT id, ?3, ?4, ?5, ?6 FROM subwallets WHERE coin_id = ?1 AND user = ?2",
            params![
                coin.0,
                user,
                transaction.dest_account.as_str(),
                transaction.amount,
                transaction.direction,
                transaction.date.as_str(),
            ],
        )
        .await?;

        Ok(())
    }
}
