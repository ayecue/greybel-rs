//! Why a script's coin, wallet, or blockchain request fails.

use greyhack_behaviour::Refusal;

/// Why a coin, wallet, or blockchain operation failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EconomyFailure {
    /// The computer has no internet access.
    NoInternet,
    /// The computer no longer exists.
    ComputerUnavailable,
    /// There is no coin with the name; contains the name.
    UnknownCoin(String),
    /// The user or password does not match.
    IncorrectCredentials,
    /// The user created no coin.
    NoCoin,
    /// The player has no bank account to pay for a wallet.
    NoBankAccount,
    /// A wallet has the user name already.
    WalletUserExists,
    /// The player has a wallet already.
    OneWalletPerPlayer,
    /// The wallet does not exist.
    UnknownWallet,
    /// The user does not own the wallet or coin.
    NotOwner,
    /// The wallet password was reset less than a day ago.
    PasswordResetTooSoon,
    /// The coin has a sub-wallet of the user already.
    SubwalletExists,
    /// The wallet's PIN does not match.
    IncorrectPin,
    /// The wallet has ten sub-wallets for the coin already.
    TooManySubwallets,
    /// No network is at the address; contains the address.
    UnknownAddress(String),
    /// The mining reward is over the maximum.
    RewardTooHigh,
    /// The mining reward is under one coin.
    RewardTooLow,
    /// The mining cycle is over the maximum.
    CycleTooLong,
    /// The mining cycle is under one hour.
    CycleTooShort,
    /// An argument breaks a rule.
    Refusal(Refusal),
}
