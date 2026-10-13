//! Economy operation errors. The runtime formats each error using the intrinsic that detects it.

use greyhack_behaviour::Refusal;

use super::blockchain::Side;

/// Why the player's purchase is not paid (`BankHelperServer.PagarCompra`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PurchaseFailure {
    /// The player has no bank account.
    NoBankAccount,
    /// The player's bank account is not in the bank.
    UnknownBankAccount(String),
    /// The account cannot pay the price after the coupons.
    InsufficientFunds,
}

/// Market order errors (`BlockchainSystem.BuyCoin` / `SellCoin`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarketFailure {
    /// No coin has that name.
    UnknownCoin(String),
    /// The order amount or price is invalid.
    Refusal(Refusal),
    /// The wallet is not registered for the coin; contains coin name and wallet address.
    MissingRegistration { coin: String, address: String },
    /// The coin has no sub-wallet with that name.
    MissingSubwallet(String),
    /// The wallet bank account does not exist.
    NoBankAccount,
    /// The bank balance is below the purchase cost in dollars, truncated to an integer.
    InsufficientMoney(i64),
    /// The sub-wallet balance is below the offered sale amount.
    InsufficientCoins,
    /// The wallet has an open order on `pending`'s side that `placed` would replace.
    OpenOffer { offer: Side, order: Side },
}

/// Immediate execution result for a placed order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trade {
    /// The entire order executed immediately.
    Complete,
    /// No part executed; the full order remains open.
    Pending,
    /// Part executed; the remainder stays open.
    Partial,
}

/// Coin transfer errors (`CoinIntrinsics.transaction`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferFailure {
    /// The coin does not exist.
    UnknownCoin,
    /// The coin has no sub-wallet with that name.
    MissingSubwallet(String),
    /// The named origin sub-wallet has no coins.
    NoCoins(String),
    /// The origin sub-wallet balance is below the transfer amount.
    InsufficientCoins,
    /// The remaining balance would be below the origin sub-wallet's pending sale offers.
    SaleOfferShortfall,
}

/// Bank transfer errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BankTransferFailure {
    /// Source and destination accounts are identical.
    SameAccount,
    /// No bank account has that number.
    UnknownAccount(String),
    /// The source account password is invalid.
    WrongPassword,
    /// The amount is too small or exceeds the source balance.
    InsufficientFunds,
}

/// Mining start and reward errors (`SubWalletIntrinsics.mining`, `BlockchainSystem.CheckBlocks`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MineJobFailure {
    /// The mining computer no longer exists.
    ComputerUnavailable,
    /// The coin does not exist.
    CoinGone,
    /// The named coin has been fully mined.
    AllMined(String),
    /// The sub-wallet does not exist.
    SubwalletGone,
    /// The sub-wallet already mines this coin on the computer.
    SecondMineJob,
    /// The computer GPU health is below 5.
    DamagedGpu,
    /// The GPU has health below 40 and failed its floating-point self-test.
    CorruptedGpu,
}
