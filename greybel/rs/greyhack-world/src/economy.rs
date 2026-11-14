//! Coins, wallets, bank accounts, and the order-book market.

pub use greyhack_world_economy::*;

/// Bank service compatibility path.
pub mod bank {
    pub use greyhack_world_economy::Bank;
}

/// Coin-script connectivity compatibility path.
pub mod coin_scripts {
    pub use greyhack_world_economy::Connectivity;
}

/// Economy failure compatibility path.
pub mod failure {
    pub use greyhack_world_economy::EconomyFailure;
}
