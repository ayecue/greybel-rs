//! Economy services, one for each game system or entity area.

pub mod bank;
pub mod coin_scripts;
pub mod coins;
pub mod failure;
pub mod market;
pub mod mirror;
pub mod prices;
pub mod rentals;
pub mod subscriptions;
pub mod subwallets;
pub mod wallets;

pub use bank::Bank;
pub use coin_scripts::Connectivity;
pub use coins::CoinService;
pub(crate) use coins::CoinServiceSupport;
pub use failure::EconomyFailure;
pub use market::Market;
pub use wallets::WalletService;
