//! Storage for bank subscriptions.

use greyhack_behaviour::SubscriptionKind;
use greyhack_world_core::{Database, Result};
use greyhack_world_storage::sql::{FromRow, RowView, execute, fetch_all, fetch_column, fetch_opt};
use turso::params;

use crate::BankSubscription;

impl FromRow for BankSubscription {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            bank: row.get("bank_number")?,
            kind: row.get_code("kind")?,
            amount: row.get("amount")?,
            next_payment_at: row.get("next_payment_at")?,
            public_ips: Vec::new(),
        })
    }
}

/// Queries of [`Database`] for economy subscriptions.
/// Queries of the tables owned by this area.
pub(crate) struct SubscriptionRepository {
    db: Database,
}

impl SubscriptionRepository {
    pub(crate) fn new(db: Database) -> Self {
        Self { db }
    }

    pub(crate) async fn subscription(
        &self,
        bank: &str,
        kind: SubscriptionKind,
    ) -> Result<Option<BankSubscription>> {
        let Some(mut subscription) = fetch_opt::<BankSubscription>(
            &self.db.connection(),
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

    pub(crate) async fn subscription_bank_of(
        &self,
        kind: SubscriptionKind,
        public_ip: &str,
    ) -> Result<Option<String>> {
        Ok(fetch_column::<String>(
            &self.db.connection(),
            "SELECT bank_number FROM bank_subscription_networks WHERE kind = ?1 AND public_ip = ?2 \
             ORDER BY bank_number LIMIT 1",
            params![kind.code(), public_ip],
        )
        .await?
        .into_iter()
        .next())
    }

    pub(crate) async fn due_subscriptions(&self, now: u64) -> Result<Vec<BankSubscription>> {
        let mut subscriptions: Vec<BankSubscription> = fetch_all(
            &self.db.connection(),
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

    pub(crate) async fn subscriptions(&self) -> Result<Vec<BankSubscription>> {
        let mut subscriptions: Vec<BankSubscription> = fetch_all(
            &self.db.connection(),
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

    pub(crate) async fn save_subscription(&self, subscription: &BankSubscription) -> Result<()> {
        execute(
            &self.db.connection(),
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
            &self.db.connection(),
            "DELETE FROM bank_subscription_networks WHERE bank_number = ?1 AND kind = ?2",
            params![subscription.bank.as_str(), subscription.kind.code()],
        )
        .await?;

        for (seq, public_ip) in subscription.public_ips.iter().enumerate() {
            execute(
                &self.db.connection(),
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

    pub(crate) async fn remove_subscription(
        &self,
        bank: &str,
        kind: SubscriptionKind,
    ) -> Result<()> {
        execute(
            &self.db.connection(),
            "DELETE FROM bank_subscription_networks WHERE bank_number = ?1 AND kind = ?2",
            params![bank, kind.code()],
        )
        .await?;
        execute(
            &self.db.connection(),
            "DELETE FROM bank_subscriptions WHERE bank_number = ?1 AND kind = ?2",
            params![bank, kind.code()],
        )
        .await?;

        Ok(())
    }
}

impl SubscriptionRepository {
    pub(crate) async fn subscription_networks(
        &self,
        bank: &str,
        kind: SubscriptionKind,
    ) -> Result<Vec<String>> {
        Ok(fetch_column(
            &self.db.connection(),
            "SELECT public_ip FROM bank_subscription_networks WHERE bank_number = ?1 AND kind = ?2 ORDER BY seq",
            params![bank, kind.code()],
        )
        .await?)
    }
}
