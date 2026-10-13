//! Bank accounts and transfers between them (`BankAccount`).

use greyhack_behaviour::{transfer_fee, valid_transfer_amount, verify_password};

use crate::{BankAccount, BankTransaction, BankTransferFailure, PersonId, Result, World};

impl World {
    /// Returns the number of the player's registered bank account (`PlayerConfigOS.GetBankAccount`).
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn player_bank(&self) -> Result<Option<String>> {
        self.bank_number_of(self.player_person).await
    }

    /// Returns the number of the bank account of `person`.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn bank_number_of(&self, person: PersonId) -> Result<Option<String>> {
        Ok(self
            .person(person)
            .await?
            .and_then(|person| person.bank_number))
    }

    /// Returns bank account `number`.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn bank_account(&self, number: &str) -> Result<Option<BankAccount>> {
        self.db.bank(number).await
    }

    /// Deposits `amount` into bank account `number` (`deposit`) and returns whether it exists.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn deposit(&mut self, number: &str, amount: f64) -> Result<bool> {
        let Some(mut bank) = self.db.bank(number).await? else {
            return Ok(false);
        };

        bank.money += amount;
        self.db.save_bank(&bank).await?;

        Ok(true)
    }

    /// Opens bank account `number` with `password` and `money` unless it already exists.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn add_bank(&mut self, number: &str, password: &str, money: f64) -> Result<()> {
        if self.db.bank(number).await?.is_none() {
            self.db.insert_bank(number, password, money).await?;
        }

        Ok(())
    }

    /// Transfers gross `amount` from `from` to `to`, deducting Grey Hack's 15% fee and recording
    /// both histories (`BankHelperServer.BankTransactionServerRpc`,
    /// `TransactionsPanel.GetFeeAmount`).
    ///
    /// # Errors
    ///
    /// Returns any database error; the inner error explains why the transfer was rejected.
    pub async fn bank_transfer(
        &mut self,
        from: &str,
        password: &str,
        to: &str,
        amount: i32,
    ) -> Result<Result<(), BankTransferFailure>> {
        if from == to {
            return Ok(Err(BankTransferFailure::SameAccount));
        }

        let Some(mut source) = self.db.bank(from).await? else {
            return Ok(Err(BankTransferFailure::UnknownAccount(from.to_owned())));
        };

        if !verify_password(&source.password_hash, password) {
            return Ok(Err(BankTransferFailure::WrongPassword));
        }

        let Some(mut destination) = self.db.bank(to).await? else {
            return Ok(Err(BankTransferFailure::UnknownAccount(to.to_owned())));
        };

        if !valid_transfer_amount(i64::from(amount)) || source.money < f64::from(amount) {
            return Ok(Err(BankTransferFailure::InsufficientFunds));
        }

        let fee = transfer_fee(i64::from(amount));
        let net = i64::from(amount) - fee;
        let date = self.date();

        source.record_transaction(BankTransaction::new(from, -fee, "Transaction fee", &date));
        source.record_transaction(BankTransaction::new(from, -net, "Money transfer", &date));
        destination.record_transaction(BankTransaction::new(to, net, "Money transfer", &date));
        self.db.save_bank(&source).await?;
        self.db.save_bank_transactions(&source).await?;
        self.db.save_bank(&destination).await?;
        self.db.save_bank_transactions(&destination).await?;

        Ok(Ok(()))
    }

    /// Moves `amount` from bank `from` to `to`, skipping accounts that do not exist.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub(crate) async fn transfer_money(&mut self, from: &str, to: &str, amount: f64) -> Result<()> {
        self.deposit(from, -amount).await?;
        self.deposit(to, amount).await?;

        Ok(())
    }
}

#[cfg(test)]
#[path = "tests/bank.rs"]
pub(in crate::systems) mod tests;
