//! Coins, bank accounts, offers, wallets, and typed economy failures.

mod blockchain;
mod failure;
mod wallet;

pub use blockchain::{
    BankAccount, BankLayout, BankSubscription, BankTransaction, Coin, CoinLayout, EconomyLayout,
    Offer, OfferLayout, PriceRecord, SubWallet, SubWalletLayout, Transaction, WalletLayout,
};
pub use failure::{BankTransferFailure, MarketFailure, PurchaseFailure, Trade, TransferFailure};
pub use wallet::{Wallet, WalletId};
