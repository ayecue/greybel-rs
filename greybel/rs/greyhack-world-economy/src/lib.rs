#![deny(clippy::await_holding_refcell_ref)]

//! The Grey Hack economy domain: bank accounts and recurring payments, coins and mining, wallets
//! and sub-wallets, and their order-book market.
//!
//! [`EconomyModule`] registers the domain schema, services, routine kinds, and lazy dependencies.
//! The composition root registers [`routes::register`] for its concrete world. Coin scripts use
//! core's lazy `NetworkGeneration` service to reach the creation domain. Public entities and services
//! are re-exported at the crate root; repositories remain private to the domain.

pub mod generation;
pub mod model;
pub mod objects;
pub mod routes;
pub mod routines;
pub mod services;
pub mod storage;

use std::rc::Rc;

pub use generation::{CoinDraft, default_coins};
pub use greyhack_behaviour::{Rng, Word};
use greyhack_world_core::WorldRef;
pub use greyhack_world_core::{CoinId, ComputerId, NetworkId, PersonId, Result, WorldError};
use greyhack_world_modules::{Module, Registry};
use greyhack_world_routines::Routine;
use greyhack_world_storage::DomainSchema;
pub use model::*;
pub use objects::{
    BlockchainHandle, CoinHandle, CoinRef, SubWalletHandle, SubWalletRef, WalletHandle,
};
pub use routines::{
    CoinRound, DailyUpkeep, MineJob, MineJobFailure, MineJobReport, MineJobService, MineJobState,
    Payment,
};
pub use services::{Bank, CoinService, Connectivity, EconomyFailure, Market, WalletService};
use storage::{
    BankRepository, CoinRepository, MineJobRepository, OfferRepository, RentalRepository,
    RoutineRepository, SubscriptionRepository, SubwalletRepository, WalletRepository,
};

/// Services and repositories of the economy domain.
#[derive(Clone)]
pub struct EconomyModule {
    /// Bank accounts, transfers, subscriptions, payments, and rentals.
    pub bank: Rc<Bank>,
    /// Coins, settings, and mining rewards.
    pub coins: Rc<CoinService>,
    /// Wallets, sub-wallets, balances, and PINs.
    pub wallets: Rc<WalletService>,
    /// Offers, trades, and prices.
    pub market: Rc<Market>,
    /// Coin mining rounds.
    pub coin_round: Rc<CoinRound>,
    /// Recurring bank payments.
    pub payment: Rc<Payment>,
    /// Daily coin-price upkeep.
    pub daily_upkeep: Rc<DailyUpkeep>,
    /// Mining subscriptions and their typed work.
    pub mine_jobs: Rc<MineJobService>,
}

impl EconomyModule {
    fn new(registry: &Rc<Registry>) -> Self {
        let world: &WorldRef = registry.world();
        let scheduler = registry.scheduler();
        let db = world.db().clone();
        let banks = Rc::new(BankRepository::new(db.clone(), registry.get()));
        let subscriptions = Rc::new(SubscriptionRepository::new(db.clone()));
        let rentals = Rc::new(RentalRepository::new(db.clone()));
        let coins = Rc::new(CoinRepository::new(db.clone(), registry.get()));
        let offers = Rc::new(OfferRepository::new(db.clone()));
        let wallet_rows = Rc::new(WalletRepository::new(db.clone(), registry.get()));
        let subwallets = Rc::new(SubwalletRepository::new(db.clone(), registry.get()));
        let mine_rows = Rc::new(MineJobRepository::new(db));
        let routine_rows = Rc::new(RoutineRepository::new(world.db().clone()));

        let bank = Rc::new(Bank::new(
            registry,
            banks.clone(),
            subscriptions,
            rentals,
            coins.clone(),
            offers.clone(),
        ));
        let wallets = Rc::new(WalletService::new(
            world,
            wallet_rows.clone(),
            coins.clone(),
            subwallets.clone(),
            offers.clone(),
            registry.get(),
            registry.get(),
        ));
        let coins_service = Rc::new(CoinService::new(
            world,
            services::CoinServiceSupport {
                coins: coins.clone(),
                subwallets: subwallets.clone(),
                computers: registry.get(),
                networks: registry.get(),
                wallets: registry.get(),
                network_generation: registry.get(),
                mine_jobs: registry.get(),
            },
        ));
        let market = Rc::new(Market::new(
            registry,
            banks,
            coins,
            wallet_rows,
            offers,
            subwallets,
        ));
        let coin_round = Rc::new(CoinRound::new(
            world,
            routines::CoinRoundSupport {
                computers: registry.get(),
                coins: registry.get(),
                wallets: registry.get(),
                mine_jobs: registry.get(),
                routines: routine_rows.clone(),
                scheduler: scheduler.clone(),
            },
        ));
        let mine_jobs = Rc::new(MineJobService::new(
            world,
            routines::MineJobServiceSupport {
                computers: registry.get(),
                coins: registry.get(),
                wallets: registry.get(),
                jobs: mine_rows,
                routines: routine_rows,
                work: registry.works(),
                coin_round: registry.get(),
            },
        ));

        Self {
            bank,
            coins: coins_service,
            wallets,
            market,
            coin_round,
            payment: Rc::new(Payment::new(world, registry.get(), scheduler.clone())),
            daily_upkeep: Rc::new(DailyUpkeep::new(world, registry.get(), scheduler)),
            mine_jobs,
        }
    }

    /// Returns the economy schema in creation order.
    #[must_use]
    pub fn schema() -> DomainSchema {
        DomainSchema::new(
            <Self as Module>::NAME,
            [
                storage::schema::GROUP,
                storage::schema::MINE_SUBSCRIBERS,
                CoinRound::tables(),
                Payment::tables(),
                DailyUpkeep::tables(),
                storage::schema::MINE_VOLATILE,
                storage::schema::MINE_ROUND_VOLATILE,
            ],
        )
    }
}

impl Module for EconomyModule {
    const NAME: &'static str = "economy";

    fn register(registry: &Rc<Registry>) -> Self {
        let module = Self::new(registry);

        registry.register_routine(module.coin_round.clone());
        registry.register_routine(module.payment.clone());
        registry.register_routine(module.daily_upkeep.clone());
        registry.register_schema(Self::schema());
        registry.register_service(module.coin_round.clone());
        registry.register_service(module.payment.clone());
        registry.register_service(module.daily_upkeep.clone());
        registry
            .register_capability::<dyn greyhack_world_modules::WorkOwner>(module.mine_jobs.clone());
        registry.register_service(module.mine_jobs.clone());
        registry.register_service(Rc::new(module.clone()));
        registry.register_service(module.bank.clone());
        registry.register_service(module.coins.clone());
        registry.register_service(module.wallets.clone());
        registry.register_service(module.market.clone());

        module
    }
}
