//! Bank accounts and transfers between them (`BankAccount`).

use greyhack_behaviour::{SubscriptionKind, transfer_fee, valid_transfer_amount, verify_password};

use crate::{
    BankAccount, BankSubscription, BankTransaction, BankTransferFailure, NetworkId, PersonId,
    PurchaseFailure, Result,
};

/// Methods of [`World`] for economy bank.
impl Bank {
    pub async fn player_bank(&self) -> Result<Option<String>> {
        self.bank_number_of(self.world.player_person()).await
    }

    pub async fn bank_number_of(&self, person: PersonId) -> Result<Option<String>> {
        self.banks.bank_number(person).await
    }

    pub async fn bank_account(&self, number: &str) -> Result<Option<BankAccount>> {
        self.banks.bank(number).await
    }

    pub async fn deposit(&self, number: &str, amount: f64) -> Result<bool> {
        let Some(mut bank) = self.banks.bank(number).await? else {
            return Ok(false);
        };

        bank.money += amount;
        self.banks.save_bank(&bank).await?;

        Ok(true)
    }

    pub async fn add_bank(&self, number: &str, password: &str, money: f64) -> Result<()> {
        if self.banks.bank(number).await?.is_none() {
            self.banks.insert_bank(number, password, money).await?;
        }

        Ok(())
    }

    pub async fn bank_transfer(
        &self,
        from: &str,
        password: &str,
        to: &str,
        amount: i32,
    ) -> Result<Result<(), BankTransferFailure>> {
        if from == to {
            return Ok(Err(BankTransferFailure::SameAccount));
        }

        let Some(mut source) = self.banks.bank(from).await? else {
            return Ok(Err(BankTransferFailure::UnknownAccount(from.to_owned())));
        };

        if !verify_password(&source.password_hash, password) {
            return Ok(Err(BankTransferFailure::WrongPassword));
        }

        let Some(mut destination) = self.banks.bank(to).await? else {
            return Ok(Err(BankTransferFailure::UnknownAccount(to.to_owned())));
        };

        if !valid_transfer_amount(i64::from(amount)) || source.money < f64::from(amount) {
            return Ok(Err(BankTransferFailure::InsufficientFunds));
        }

        let fee = transfer_fee(i64::from(amount));
        let net = i64::from(amount) - fee;
        let date = self.world.date();

        source.record_transaction(BankTransaction::new(from, -fee, "Transaction fee", &date));
        source.record_transaction(BankTransaction::new(from, -net, "Money transfer", &date));
        destination.record_transaction(BankTransaction::new(to, net, "Money transfer", &date));
        self.banks.save_bank(&source).await?;
        self.banks.save_bank_transactions(&source).await?;
        self.banks.save_bank(&destination).await?;
        self.banks.save_bank_transactions(&destination).await?;

        Ok(Ok(()))
    }

    pub async fn transfer_money(&self, from: &str, to: &str, amount: f64) -> Result<()> {
        self.deposit(from, -amount).await?;
        self.deposit(to, amount).await?;

        Ok(())
    }

    /// `BankHelperServer.PagarCompra`: records a purchase without shop-coupon handling.
    pub async fn pay_purchase(
        &self,
        price: i64,
        reason: &str,
        _coupons: bool,
    ) -> Result<std::result::Result<(), PurchaseFailure>> {
        let Some(number) = self.player_bank().await? else {
            return Ok(Err(PurchaseFailure::NoBankAccount));
        };
        let Some(mut bank) = self.banks.bank(&number).await? else {
            return Ok(Err(PurchaseFailure::UnknownBankAccount(number)));
        };

        if bank.money < price as f64 {
            return Ok(Err(PurchaseFailure::InsufficientFunds));
        }

        bank.record_transaction(BankTransaction::payment(
            price,
            reason,
            &self.world.date(),
            true,
        ));
        self.banks.save_bank(&bank).await?;
        self.banks.save_bank_transactions(&bank).await?;

        Ok(Ok(()))
    }
}

/// A world service with only its declared dependencies.
pub struct Bank {
    pub(crate) world: greyhack_world_core::WorldRef,
    pub(crate) banks: std::rc::Rc<crate::storage::BankRepository>,
    pub(crate) subscriptions: std::rc::Rc<crate::storage::SubscriptionRepository>,
    pub(crate) rentals: std::rc::Rc<crate::storage::RentalRepository>,
    pub(crate) coins: std::rc::Rc<crate::storage::CoinRepository>,
    pub(crate) offers: std::rc::Rc<crate::storage::OfferRepository>,
    pub(crate) computers: greyhack_world_core::Dep<greyhack_world_core::ComputerService>,
    pub(crate) networks: greyhack_world_core::Dep<greyhack_world_core::NetworkService>,
    pub(crate) wallets: greyhack_world_core::Dep<crate::services::WalletService>,
    pub(crate) mine_jobs: std::rc::Rc<greyhack_world_modules::WorkQueue>,
    pub(crate) scheduler: std::rc::Rc<greyhack_world_routines::Scheduler>,
}

impl Bank {
    pub(crate) fn new(
        registry: &std::rc::Rc<greyhack_world_modules::Registry>,
        banks: std::rc::Rc<crate::storage::BankRepository>,
        subscriptions: std::rc::Rc<crate::storage::SubscriptionRepository>,
        rentals: std::rc::Rc<crate::storage::RentalRepository>,
        coins: std::rc::Rc<crate::storage::CoinRepository>,
        offers: std::rc::Rc<crate::storage::OfferRepository>,
    ) -> Self {
        Self {
            world: registry.world::<greyhack_world_core::WorldRef>().clone(),
            banks,
            subscriptions,
            rentals,
            coins,
            offers,
            wallets: registry.get(),
            computers: registry.get(),
            networks: registry.get(),
            mine_jobs: registry.works(),
            scheduler: registry.scheduler(),
        }
    }
}

impl Bank {
    /// Returns the bank account number linked to `person`.
    pub async fn bank_number(&self, person: PersonId) -> Result<Option<String>> {
        self.banks.bank_number(person).await
    }
}

impl Bank {
    /// Links `person` to `number`, or removes that link when `number` is `None`.
    pub async fn set_bank_number(&self, person: PersonId, number: Option<&str>) -> Result<()> {
        self.banks.set_bank_number(person, number).await
    }
}

impl Bank {
    /// Returns bank account `number`, including its transaction history.
    pub async fn bank(&self, number: &str) -> Result<Option<BankAccount>> {
        self.banks.bank(number).await
    }
}

impl Bank {
    /// Writes the account password hash and balance.
    pub async fn save_bank(&self, bank: &BankAccount) -> Result<()> {
        self.banks.save_bank(bank).await
    }
}

impl Bank {
    /// Replaces the account transaction history with `bank.transactions`.
    pub async fn save_bank_transactions(&self, bank: &BankAccount) -> Result<()> {
        self.banks.save_bank_transactions(bank).await
    }
}

impl Bank {
    /// Returns the account subscription of `kind`, if one exists.
    pub async fn subscription(
        &self,
        bank: &str,
        kind: SubscriptionKind,
    ) -> Result<Option<BankSubscription>> {
        self.subscriptions.subscription(bank, kind).await
    }
}

impl Bank {
    /// Returns the administrator used in rental pricing.
    pub async fn rental_admin(&self, network: NetworkId) -> Result<Option<PersonId>> {
        self.rentals.rental_admin(network).await
    }
}

impl Bank {
    /// Changes the administrator used in rental pricing.
    pub async fn set_rental_admin(
        &self,
        network: NetworkId,
        admin: Option<PersonId>,
    ) -> Result<()> {
        self.rentals.set_rental_admin(network, admin).await
    }
}
