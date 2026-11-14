//! Storage for bank accounts and their transactions.

use greyhack_world_core::Database;
use greyhack_world_storage::sql::{FromRow, RowView, execute, fetch_all, fetch_opt, fetch_value};
use turso::params;

use crate::{BankAccount, BankTransaction, PersonId, Result};

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

/// Queries of [`Database`] for economy banks.
/// Queries of the tables owned by this area.
pub(crate) struct BankRepository {
    db: Database,
    computers: greyhack_world_core::Dep<greyhack_world_core::ComputerService>,
}

impl BankRepository {
    pub(crate) fn new(
        db: Database,
        computers: greyhack_world_core::Dep<greyhack_world_core::ComputerService>,
    ) -> Self {
        Self { db, computers }
    }

    pub(crate) async fn bank_number(&self, person: PersonId) -> Result<Option<String>> {
        Ok(fetch_value(
            &self.db.connection(),
            "SELECT bank_number FROM person_banks WHERE person_id = ?1",
            params![person.0],
        )
        .await?)
    }

    pub(crate) async fn set_bank_number(
        &self,
        person: PersonId,
        number: Option<&str>,
    ) -> Result<()> {
        match number {
            Some(number) => {
                execute(
                    &self.db.connection(),
                    "INSERT INTO person_banks (person_id, bank_number) VALUES (?1, ?2) \
                     ON CONFLICT (person_id) DO UPDATE SET bank_number = ?2",
                    params![person.0, number],
                )
                .await?
            }
            None => {
                execute(
                    &self.db.connection(),
                    "DELETE FROM person_banks WHERE person_id = ?1",
                    params![person.0],
                )
                .await?
            }
        };

        Ok(())
    }

    pub(crate) async fn bank(&self, number: &str) -> Result<Option<BankAccount>> {
        let bank: Option<BankAccount> = fetch_opt(
            &self.db.connection(),
            "SELECT number, password_hash, money FROM bank_accounts WHERE number = ?1",
            params![number],
        )
        .await?;
        let Some(mut bank) = bank else {
            return Ok(None);
        };

        bank.transactions = fetch_all(
            &self.db.connection(),
            "SELECT account, amount, reason, date, success FROM bank_transactions \
             WHERE bank_number = ?1 ORDER BY id",
            params![number],
        )
        .await?;

        Ok(Some(bank))
    }

    pub(crate) async fn insert_bank(&self, number: &str, password: &str, money: f64) -> Result<()> {
        let hash = self.computers.register_password(password).await?;

        execute(
            &self.db.connection(),
            "INSERT OR REPLACE INTO bank_accounts (number, password_hash, money) VALUES (?1, ?2, ?3)",
            params![number, hash.as_str(), money],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn save_bank(&self, bank: &BankAccount) -> Result<()> {
        execute(
            &self.db.connection(),
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

    pub(crate) async fn save_bank_transactions(&self, bank: &BankAccount) -> Result<()> {
        execute(
            &self.db.connection(),
            "DELETE FROM bank_transactions WHERE bank_number = ?1",
            params![bank.number.as_str()],
        )
        .await?;

        for transaction in &bank.transactions {
            execute(
                &self.db.connection(),
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
