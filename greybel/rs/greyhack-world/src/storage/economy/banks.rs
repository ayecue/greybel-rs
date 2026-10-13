//! Storage for bank accounts and their transactions.

use turso::params;

use super::super::Database;
use super::super::sql::{FromRow, RowView, execute, fetch_all, fetch_opt};
use crate::{BankAccount, BankTransaction, Result};

impl FromRow for BankAccount {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            number: row.get("number")?,
            password_hash: row.get("password_hash")?,
            money: row.get("money")?,
            transactions: Vec::new(),
        })
    }
}

impl FromRow for BankTransaction {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            account: row.get("account")?,
            amount: row.get("amount")?,
            reason: row.get("reason")?,
            date: row.get("date")?,
            success: row.get("success")?,
        })
    }
}

impl Database {
    /// Returns bank account `number` with its stored transactions, at most the 30 newest.
    pub(crate) async fn bank(&self, number: &str) -> Result<Option<BankAccount>> {
        let bank: Option<BankAccount> = fetch_opt(
            &self.conn,
            "SELECT number, password_hash, money FROM bank_accounts WHERE number = ?1",
            params![number],
        )
        .await?;
        let Some(mut bank) = bank else {
            return Ok(None);
        };

        bank.transactions = fetch_all(
            &self.conn,
            "SELECT account, amount, reason, date, success FROM bank_transactions \
             WHERE bank_number = ?1 ORDER BY id",
            params![number],
        )
        .await?;

        Ok(Some(bank))
    }

    /// Opens or replaces bank account `number` with `password` and `money`.
    pub(crate) async fn insert_bank(&self, number: &str, password: &str, money: f64) -> Result<()> {
        let hash = self.register_password(password).await?;

        execute(
            &self.conn,
            "INSERT OR REPLACE INTO bank_accounts (number, password_hash, money) VALUES (?1, ?2, ?3)",
            params![number, hash.as_str(), money],
        )
        .await?;

        Ok(())
    }

    /// Updates bank password and balance.
    pub(crate) async fn save_bank(&self, bank: &BankAccount) -> Result<()> {
        execute(
            &self.conn,
            "UPDATE bank_accounts SET password_hash = ?2, money = ?3 WHERE number = ?1",
            params![
                bank.number.as_str(),
                bank.password_hash.as_str(),
                bank.money
            ],
        )
        .await?;

        Ok(())
    }

    /// Replaces bank transaction history with its ten retained entries.
    pub(crate) async fn save_bank_transactions(&self, bank: &BankAccount) -> Result<()> {
        execute(
            &self.conn,
            "DELETE FROM bank_transactions WHERE bank_number = ?1",
            params![bank.number.as_str()],
        )
        .await?;

        for transaction in &bank.transactions {
            execute(
                &self.conn,
                "INSERT INTO bank_transactions (bank_number, account, amount, reason, date, success) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    bank.number.as_str(),
                    transaction.account.as_str(),
                    transaction.amount,
                    transaction.reason.as_str(),
                    transaction.date.as_str(),
                    transaction.success,
                ],
            )
            .await?;
        }

        Ok(())
    }
}
