//! Database-backed coins, sub-wallets, price history, order books, and bank accounts.

use greyhack_behaviour::{
    GAME_ACCOUNT, MAX_BANK_TRANSACTIONS, Side, SubscriptionKind, verify_password,
};
use greyhack_world_core::CoinId;

use crate::WalletId;

/// Global economy state mirrored from another world without its numeric IDs.
#[derive(Debug, Clone, Default)]
pub struct EconomyLayout {
    pub banks: Vec<BankLayout>,
    pub subscriptions: Vec<BankSubscription>,
    pub coins: Vec<CoinLayout>,
    pub wallets: Vec<WalletLayout>,
    pub offers: Vec<OfferLayout>,
}

/// Mirrored bank account with the plaintext password required by the password registry.
#[derive(Debug, Clone)]
pub struct BankLayout {
    pub number: String,
    pub password: String,
    pub money: f64,
}

/// Mirrored coin and price history.
#[derive(Debug, Clone)]
pub struct CoinLayout {
    pub name: String,
    pub creator_user: String,
    pub creator_password: String,
    pub address: String,
    pub cycle_hours: f64,
    pub reward: f64,
    pub mined_coins: f64,
    pub max_supply: f64,
    pub price: f64,
    pub prices: Vec<PriceRecord>,
}

/// Mirrored wallet and sub-wallets.
#[derive(Debug, Clone)]
pub struct WalletLayout {
    pub user: String,
    pub password: String,
    pub pin: String,
    pub bank: String,
    pub subwallets: Vec<SubWalletLayout>,
}

/// Mirrored sub-wallet that references coin and wallet names instead of old IDs.
#[derive(Debug, Clone)]
pub struct SubWalletLayout {
    pub coin: String,
    pub user: String,
    pub password: String,
    pub balance: f64,
    pub info: String,
    pub wallet_user: String,
}

/// Mirrored standing offer that references coin and wallet names instead of old IDs.
#[derive(Debug, Clone)]
pub struct OfferLayout {
    pub coin: String,
    pub wallet_user: String,
    /// Sub-wallet credited or debited by the offer (`HelperOffer.subwalletID`).
    pub subwallet: String,
    pub side: Side,
    pub price: f64,
    pub amount: f64,
}

/// Coin deposit account (`BlockchainSystem.SubWallet`).
#[derive(Debug, Clone, PartialEq)]
pub struct SubWallet {
    pub coin: CoinId,
    pub user: String,
    pub password_hash: String,
    pub balance: f64,
    pub info: String,
    /// Username of the wallet that created the sub-wallet.
    pub wallet_user: String,
}

/// Most recent sub-wallet transfer (`last_transaction`).
#[derive(Debug, Clone, PartialEq)]
pub struct Transaction {
    pub dest_account: String,
    pub amount: f64,
    /// Transfer direction: 0 for incoming, 1 for outgoing.
    pub direction: u8,
    pub date: String,
}

/// Historical coin price with `dd/MMM/yyyy - HH:mm` timestamp.
#[derive(Debug, Clone, PartialEq)]
pub struct PriceRecord {
    pub price: f64,
    pub date: String,
}

/// Bank-account credit or debit (`BankAccount.Transaccion`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BankTransaction {
    pub account: String,
    pub amount: i64,
    pub reason: String,
    pub date: String,
    /// Failed payments remain in history without changing the balance.
    pub success: bool,
}

impl BankTransaction {
    /// Returns a transaction of `amount` dollars with `account` that went through.
    #[must_use]
    pub fn new(account: &str, amount: i64, reason: &str, date: &str) -> Self {
        Self {
            account: account.to_owned(),
            amount,
            reason: reason.to_owned(),
            date: date.to_owned(),
            success: true,
        }
    }

    /// Returns a payment of `amount` dollars to the game (`BankHelperServer.PagarCompra`,
    /// `BankSubs.Payment.ProcessPayment`) that did or did not go through, according to `success`.
    #[must_use]
    pub fn payment(amount: i64, reason: &str, date: &str, success: bool) -> Self {
        Self {
            success,
            ..Self::new(GAME_ACCOUNT, -amount, reason, date)
        }
    }
}

/// Standing buy or sell order (`BlockchainSystem.HelperOffer`), settled through the wallet bank
/// account.
#[derive(Debug, Clone, PartialEq)]
pub struct Offer {
    pub coin: CoinId,
    pub wallet: WalletId,
    /// Sub-wallet credited or debited by the offer (`HelperOffer.subwalletID`).
    pub subwallet: String,
    pub side: Side,
    pub price: f64,
    pub amount: f64,
}

impl Offer {
    /// Returns market-listing text in `price amount` format.
    #[must_use]
    pub fn describe(&self) -> String {
        format!("{} {}", self.price as i64, self.amount as i64)
    }
}

/// Coin model (`BlockchainSystem.Coin`).
#[derive(Debug, Clone, PartialEq)]
pub struct Coin {
    /// Stable identifier retained across deletion and database reloads.
    pub id: CoinId,
    pub name: String,
    pub creator_user: String,
    pub creator_password_hash: String,
    pub address: String,
    /// Hours between mining rounds.
    pub cycle_hours: f64,
    /// World time of the last mining round; `None` before the immediately due first round.
    pub last_cycle_at: Option<u64>,
    pub reward: f64,
    pub mined_coins: f64,
    pub max_supply: f64,
    pub price: f64,
}

impl Coin {
    /// Returns whether `user` and `password` match the creator credentials.
    #[must_use]
    pub fn verify_creator(&self, user: &str, password: &str) -> bool {
        self.creator_user == user && verify_password(&self.creator_password_hash, password)
    }
}

/// Monthly bank-account charge (`BankSubs.Payment`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BankSubscription {
    pub bank: String,
    pub kind: SubscriptionKind,
    pub amount: i64,
    /// World time of the next scheduled payment.
    pub next_payment_at: u64,
    /// Public addresses of the rented networks it charges for, in the order they were added
    /// (`BankSubs.Payment.publicIps`). Only rental subscriptions list networks.
    pub public_ips: Vec<String>,
}

/// Bank account (`BankAccount`).
#[derive(Debug, Clone, PartialEq)]
pub struct BankAccount {
    pub number: String,
    pub password_hash: String,
    pub money: f64,
    /// Oldest first, limited to the 30 newest entries (`BankAccount.DeleteOldTransactions`).
    pub transactions: Vec<BankTransaction>,
}

impl BankAccount {
    /// Applies a transaction using `BankAccount.Ingreso` semantics and retains only the 30 newest
    /// entries (`BankAccount.DeleteOldTransactions`).
    pub fn record_transaction(&mut self, transaction: BankTransaction) {
        if transaction.success {
            self.money += transaction.amount as f64;
        }

        self.transactions.push(transaction);

        let excess = self
            .transactions
            .len()
            .saturating_sub(MAX_BANK_TRANSACTIONS);

        self.transactions.drain(..excess);
    }
}
