//! Economy repositories and the tables they own.

mod banks;
mod coins;
mod mine_jobs;
mod offers;
mod rentals;
mod routines;
pub mod schema;
mod subscriptions;
mod subwallets;
mod wallets;

pub(crate) use banks::BankRepository;
pub(crate) use coins::CoinRepository;
pub(crate) use mine_jobs::MineJobRepository;
pub(crate) use offers::OfferRepository;
pub(crate) use rentals::RentalRepository;
pub(crate) use routines::RoutineRepository;
pub(crate) use subscriptions::SubscriptionRepository;
pub(crate) use subwallets::SubwalletRepository;
pub(crate) use wallets::WalletRepository;
