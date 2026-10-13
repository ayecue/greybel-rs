//! Storage for bank subscriptions.

use greyhack_behaviour::SubscriptionKind;
use turso::params;

use super::super::Database;
use super::super::sql::{FromRow, RowView, execute, fetch_all, fetch_column, fetch_opt};
use crate::{BankSubscription, Result};

impl FromRow for BankSubscription {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            bank: row.get("bank_number")?,
            kind: row.get("kind")?,
            amount: row.get("amount")?,
            next_payment_at: row.get("next_payment_at")?,
            public_ips: Vec::new(),
        })
    }
}

impl Database {
    /// Returns monthly subscription `kind` on `bank`.
    pub(crate) async fn subscription(
        &self,
        bank: &str,
        kind: SubscriptionKind,
    ) -> Result<Option<BankSubscription>> {
        let Some(mut subscription) = fetch_opt::<BankSubscription>(
            &self.conn,
            "SELECT bank_number, kind, amount, next_payment_at FROM bank_subscriptions \
             WHERE bank_number = ?1 AND kind = ?2",
            params![bank, kind.code()],
        )
        .await?
        else {
            return Ok(None);
        };

        subscription.public_ips = self.subscription_networks(bank, kind).await?;

        Ok(Some(subscription))
    }

    /// Returns the public addresses subscription `kind` on `bank` charges for, in the order they were added.
    async fn subscription_networks(
        &self,
        bank: &str,
        kind: SubscriptionKind,
    ) -> Result<Vec<String>> {
        Ok(fetch_column(
            &self.conn,
            "SELECT public_ip FROM bank_subscription_networks WHERE bank_number = ?1 AND kind = ?2 ORDER BY seq",
            params![bank, kind.code()],
        )
        .await?)
    }

    /// Returns the bank account whose subscription `kind` charges for the network at `public_ip`.
    pub(crate) async fn subscription_bank_of(
        &self,
        kind: SubscriptionKind,
        public_ip: &str,
    ) -> Result<Option<String>> {
        Ok(fetch_column::<String>(
            &self.conn,
            "SELECT bank_number FROM bank_subscription_networks WHERE kind = ?1 AND public_ip = ?2 \
             ORDER BY bank_number LIMIT 1",
            params![kind.code(), public_ip],
        )
        .await?
        .into_iter()
        .next())
    }

    /// Returns all monthly subscriptions due at `now`.
    pub(crate) async fn due_subscriptions(&self, now: u64) -> Result<Vec<BankSubscription>> {
        let mut subscriptions: Vec<BankSubscription> = fetch_all(
            &self.conn,
            "SELECT bank_number, kind, amount, next_payment_at FROM bank_subscriptions \
             WHERE next_payment_at <= ?1 ORDER BY bank_number, kind",
            params![now],
        )
        .await?;

        for subscription in &mut subscriptions {
            subscription.public_ips = self
                .subscription_networks(&subscription.bank, subscription.kind)
                .await?;
        }

        Ok(subscriptions)
    }

    /// Returns all monthly subscriptions.
    pub(crate) async fn subscriptions(&self) -> Result<Vec<BankSubscription>> {
        let mut subscriptions: Vec<BankSubscription> = fetch_all(
            &self.conn,
            "SELECT bank_number, kind, amount, next_payment_at FROM bank_subscriptions \
             ORDER BY bank_number, kind",
            (),
        )
        .await?;

        for subscription in &mut subscriptions {
            subscription.public_ips = self
                .subscription_networks(&subscription.bank, subscription.kind)
                .await?;
        }

        Ok(subscriptions)
    }

    /// Inserts or updates a monthly subscription and the addresses it charges for.
    pub(crate) async fn save_subscription(&self, subscription: &BankSubscription) -> Result<()> {
        execute(
            &self.conn,
            "INSERT OR REPLACE INTO bank_subscriptions (bank_number, kind, amount, next_payment_at) \
             VALUES (?1, ?2, ?3, ?4)",
            params![
                subscription.bank.as_str(),
                subscription.kind.code(),
                subscription.amount,
                subscription.next_payment_at,
            ],
        )
        .await?;
        execute(
            &self.conn,
            "DELETE FROM bank_subscription_networks WHERE bank_number = ?1 AND kind = ?2",
            params![subscription.bank.as_str(), subscription.kind.code()],
        )
        .await?;

        for (seq, public_ip) in subscription.public_ips.iter().enumerate() {
            execute(
                &self.conn,
                "INSERT INTO bank_subscription_networks (bank_number, kind, seq, public_ip) \
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    subscription.bank.as_str(),
                    subscription.kind.code(),
                    seq as u64,
                    public_ip.as_str(),
                ],
            )
            .await?;
        }

        Ok(())
    }

    /// Deletes a monthly subscription.
    pub(crate) async fn remove_subscription(
        &self,
        bank: &str,
        kind: SubscriptionKind,
    ) -> Result<()> {
        execute(
            &self.conn,
            "DELETE FROM bank_subscription_networks WHERE bank_number = ?1 AND kind = ?2",
            params![bank, kind.code()],
        )
        .await?;
        execute(
            &self.conn,
            "DELETE FROM bank_subscriptions WHERE bank_number = ?1 AND kind = ?2",
            params![bank, kind.code()],
        )
        .await?;

        Ok(())
    }
}
