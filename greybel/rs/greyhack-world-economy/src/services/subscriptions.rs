//! Monthly bank subscriptions and their payments (`BankSubs`).

use greyhack_behaviour::{
    NetworkEffect, RENTAL_ADMIN_PRICE, SubscriptionKind, add_month, admin_payment_ends,
    payment_ends,
};

use crate::{Bank, BankSubscription, BankTransaction, Payment, Result};

/// Methods of [`World`] for economy subscriptions.
impl Bank {
    pub async fn add_subscription(
        &self,
        bank: &str,
        kind: SubscriptionKind,
        amount: i64,
        public_ip: &str,
    ) -> Result<bool> {
        if self.banks.bank(bank).await?.is_none() {
            return Ok(false);
        }

        if let Some(mut subscription) = self.subscriptions.subscription(bank, kind).await? {
            // `BankSubs.Payment.AddMoneyPayment`
            subscription.amount += amount;

            if !public_ip.is_empty() && !subscription.public_ips.iter().any(|ip| ip == public_ip) {
                subscription.public_ips.push(public_ip.to_owned());
            }

            self.store_subscription(&subscription, payment_ends(subscription.amount))
                .await?;
        } else {
            let subscription = BankSubscription {
                bank: bank.to_owned(),
                kind,
                amount,
                next_payment_at: add_month(self.world.time_seconds()),
                public_ips: if public_ip.is_empty() {
                    Vec::new()
                } else {
                    vec![public_ip.to_owned()]
                },
            };

            self.subscriptions.save_subscription(&subscription).await?;
            self.schedule_payment(&subscription).await?;
        }

        Ok(true)
    }

    pub async fn store_subscription(
        &self,
        subscription: &BankSubscription,
        ends: bool,
    ) -> Result<()> {
        if ends {
            // `BankSubs.StopAndRemovePayment`
            Payment::cancel_with(
                &self.world,
                &self.scheduler,
                &subscription.bank,
                subscription.kind,
            )
            .await?;

            self.subscriptions
                .remove_subscription(&subscription.bank, subscription.kind)
                .await
        } else {
            self.subscriptions.save_subscription(subscription).await
        }
    }

    pub async fn remove_admin_payment(&self, bank: &str) -> Result<bool> {
        let Some(mut subscription) = self
            .subscriptions
            .subscription(bank, SubscriptionKind::Admin)
            .await?
        else {
            return Ok(false);
        };

        subscription.amount -= RENTAL_ADMIN_PRICE;

        self.store_subscription(&subscription, admin_payment_ends(subscription.amount))
            .await?;

        Ok(true)
    }

    pub async fn remove_rental_payments(
        &self,
        bank: &str,
        public_ip: &str,
        rental_price: i64,
        has_admin: bool,
    ) -> Result<bool> {
        let Some(mut rental) = self
            .subscriptions
            .subscription(bank, SubscriptionKind::Rented)
            .await?
        else {
            return Ok(false);
        };

        // `BankSubs.Payment.RemoveMoneyPayment`
        rental.amount -= rental_price;

        if let Some(index) = rental.public_ips.iter().position(|ip| ip == public_ip) {
            rental.public_ips.remove(index);
        }

        self.store_subscription(&rental, payment_ends(rental.amount))
            .await?;

        if has_admin {
            let Some(mut admin) = self
                .subscriptions
                .subscription(bank, SubscriptionKind::Admin)
                .await?
            else {
                return Ok(false);
            };

            admin.amount -= RENTAL_ADMIN_PRICE;

            self.store_subscription(&admin, payment_ends(admin.amount))
                .await?;
        }

        Ok(true)
    }

    pub async fn bank_subscription(
        &self,
        bank: &str,
        kind: SubscriptionKind,
    ) -> Result<Option<BankSubscription>> {
        self.subscriptions.subscription(bank, kind).await
    }

    pub async fn bank_subscriptions(&self) -> Result<Vec<BankSubscription>> {
        self.subscriptions.subscriptions().await
    }

    pub async fn schedule_payment(&self, subscription: &BankSubscription) -> Result<()> {
        Payment::schedule_with(
            &self.world,
            &self.scheduler,
            subscription.next_payment_at,
            &subscription.bank,
            subscription.kind,
        )
        .await?;

        Ok(())
    }

    pub async fn restore_payments(&self) -> Result<()> {
        for mut subscription in self.subscriptions.subscriptions().await? {
            if subscription.next_payment_at < self.world.time_seconds() {
                subscription.next_payment_at = add_month(self.world.time_seconds());
                self.subscriptions.save_subscription(&subscription).await?;
            }

            self.schedule_payment(&subscription).await?;
        }

        Ok(())
    }

    pub async fn run_subscription_payments(&self) -> Result<()> {
        for subscription in self
            .subscriptions
            .due_subscriptions(self.world.time_seconds())
            .await?
        {
            self.run_payment(&subscription.bank, subscription.kind)
                .await?;
        }

        Ok(())
    }

    pub async fn run_payment(&self, bank: &str, kind: SubscriptionKind) -> Result<()> {
        let Some(mut subscription) = self.subscriptions.subscription(bank, kind).await? else {
            return Ok(());
        };

        subscription.next_payment_at = add_month(subscription.next_payment_at);
        self.subscriptions.save_subscription(&subscription).await?;

        let Some(mut account) = self.banks.bank(bank).await? else {
            Payment::cancel_with(&self.world, &self.scheduler, bank, kind).await?;

            return Ok(());
        };

        // `BankSubs.Payment.SecondsNextPayment`: a date in the past becomes one month from now.
        if subscription.next_payment_at < self.world.time_seconds() {
            subscription.next_payment_at = add_month(self.world.time_seconds());
            self.subscriptions.save_subscription(&subscription).await?;
        }

        self.schedule_payment(&subscription).await?;

        let success = account.money >= subscription.amount as f64;

        account.record_transaction(BankTransaction::payment(
            subscription.amount,
            "Monthly Payment",
            &self.world.date(),
            success,
        ));
        self.banks.save_bank(&account).await?;
        self.banks.save_bank_transactions(&account).await?;

        let disable = match subscription.kind.network_effect(success) {
            NetworkEffect::Skip => return Ok(()),
            NetworkEffect::Enable => false,
            NetworkEffect::Disable => true,
        };

        for public_ip in &subscription.public_ips {
            self.networks
                .settings
                .disable_network(public_ip, disable)
                .await?;
        }

        Ok(())
    }
}
