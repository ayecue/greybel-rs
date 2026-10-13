//! Monthly bank subscriptions and their payments (`BankSubs`).

use greyhack_behaviour::{
    NetworkEffect, RENTAL_ADMIN_PRICE, SubscriptionKind, add_month, admin_payment_ends,
    payment_ends,
};

use crate::{BankSubscription, BankTransaction, Result, World};

impl World {
    /// Adds `amount` to monthly subscription `kind` on `bank` and schedules the next payment one
    /// calendar month later (`BankSubs.AddPayment`). A new subscription is made even when `amount`
    /// is not positive. Adding to an existing one retains its account and next date, lists
    /// `public_ip` when it is not empty and not listed yet, and removes the subscription when its
    /// amount becomes 0. Returns false when `bank` does not exist.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn add_subscription(
        &mut self,
        bank: &str,
        kind: SubscriptionKind,
        amount: i64,
        public_ip: &str,
    ) -> Result<bool> {
        if self.db.bank(bank).await?.is_none() {
            return Ok(false);
        }

        if let Some(mut subscription) = self.db.subscription(bank, kind).await? {
            // `BankSubs.Payment.AddMoneyPayment`
            subscription.amount += amount;

            if !public_ip.is_empty() && !subscription.public_ips.iter().any(|ip| ip == public_ip) {
                subscription.public_ips.push(public_ip.to_owned());
            }

            self.store_subscription(&subscription, payment_ends(subscription.amount))
                .await?;
        } else {
            self.db
                .save_subscription(&BankSubscription {
                    bank: bank.to_owned(),
                    kind,
                    amount,
                    next_payment_at: add_month(self.time_seconds),
                    public_ips: if public_ip.is_empty() {
                        Vec::new()
                    } else {
                        vec![public_ip.to_owned()]
                    },
                })
                .await?;
        }

        Ok(true)
    }

    /// Saves `subscription`, or removes it when it `ends`.
    async fn store_subscription(
        &mut self,
        subscription: &BankSubscription,
        ends: bool,
    ) -> Result<()> {
        if ends {
            self.db
                .remove_subscription(&subscription.bank, subscription.kind)
                .await
        } else {
            self.db.save_subscription(subscription).await
        }
    }

    /// Takes a hired administrator's charge off the admin subscription of `bank`, removing it once
    /// nothing is left (`BankSubs.RemoveAdminPayment`). Returns false when `bank` has no admin
    /// subscription.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn remove_admin_payment(&mut self, bank: &str) -> Result<bool> {
        let Some(mut subscription) = self.db.subscription(bank, SubscriptionKind::Admin).await?
        else {
            return Ok(false);
        };

        subscription.amount -= RENTAL_ADMIN_PRICE;

        self.store_subscription(&subscription, admin_payment_ends(subscription.amount))
            .await?;

        Ok(true)
    }

    /// Takes a deleted rented network's charges off the subscriptions of `bank`: `rental_price` and
    /// the network's address off the rental subscription, and, when the network had a hired
    /// administrator, the administrator's charge off the admin subscription
    /// (`BankSubs.RemoveRentalPayments`). A subscription whose amount becomes 0 is removed.
    /// Returns false when a subscription it needs is missing: the game raises an error there, after
    /// the rental subscription was already changed when it is the admin subscription that is
    /// missing.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn remove_rental_payments(
        &mut self,
        bank: &str,
        public_ip: &str,
        rental_price: i64,
        has_admin: bool,
    ) -> Result<bool> {
        let Some(mut rental) = self.db.subscription(bank, SubscriptionKind::Rented).await? else {
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
            let Some(mut admin) = self.db.subscription(bank, SubscriptionKind::Admin).await? else {
                return Ok(false);
            };

            admin.amount -= RENTAL_ADMIN_PRICE;

            self.store_subscription(&admin, payment_ends(admin.amount))
                .await?;
        }

        Ok(true)
    }

    /// Returns monthly subscription `kind` on `bank`.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn bank_subscription(
        &self,
        bank: &str,
        kind: SubscriptionKind,
    ) -> Result<Option<BankSubscription>> {
        self.db.subscription(bank, kind).await
    }

    /// Returns every monthly subscription, ordered by bank account and kind.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn bank_subscriptions(&self) -> Result<Vec<BankSubscription>> {
        self.db.subscriptions().await
    }

    /// Applies due monthly payments and advances their next dates (`BankSubs.Payment.ProcessPayment`).
    /// A payment the account cannot cover is recorded as failed without changing the balance. Paid
    /// rental charges enable the networks their subscription lists and unpaid ones disable them
    /// (`BankSubs.Payment.DisableNetwork`). An unpaid admin charge changes nothing more: the game
    /// removes administrators only of networks listed on the admin subscription, which it never
    /// lists. The game stops a player's payments when the player has not connected for over fifteen
    /// days; Greybel's player is always connected.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn run_subscription_payments(&mut self) -> Result<()> {
        let subscriptions = self.db.due_subscriptions(self.time_seconds).await?;

        for mut subscription in subscriptions {
            subscription.next_payment_at = add_month(subscription.next_payment_at);
            self.db.save_subscription(&subscription).await?;

            let Some(mut bank) = self.db.bank(&subscription.bank).await? else {
                continue;
            };
            let success = bank.money >= subscription.amount as f64;

            bank.record_transaction(BankTransaction::payment(
                subscription.amount,
                "Monthly Payment",
                &self.date(),
                success,
            ));
            self.db.save_bank(&bank).await?;
            self.db.save_bank_transactions(&bank).await?;

            let disable = match subscription.kind.network_effect(success) {
                NetworkEffect::Skip => continue,
                NetworkEffect::Enable => false,
                NetworkEffect::Disable => true,
            };

            for public_ip in &subscription.public_ips {
                self.disable_network(public_ip, disable).await?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
#[path = "tests/subscriptions.rs"]
mod tests;
