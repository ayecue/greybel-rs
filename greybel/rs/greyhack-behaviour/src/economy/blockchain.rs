//! Coin prices, mining rewards, bank transfers, and the charges of monthly bank payments
//! (`BlockchainSystem`, `Coin`, `BankSubs`, `BankHelperServer`).

use crate::{Rng, SECONDS_PER_HOUR};

/// Percent of a bank transfer the bank takes as its fee (`TransactionsPanel.GetFeeAmount`).
pub const TRANSFER_FEE_PERCENT: i64 = 15;
/// Transactions a bank account stores (`BankAccount.DeleteOldTransactions`).
pub const MAX_BANK_TRANSACTIONS: usize = 30;
/// Price records a coin stores (`Coin.UpdateMoneyValue`).
pub const PRICE_HISTORY_LIMIT: usize = 7;
/// Account a bank transaction names when the other side is the game rather than another account.
pub const GAME_ACCOUNT: &str = "Unknown";
/// Coupons a new player has (`PlayerConfigOS.cupones`).
pub const INITIAL_COUPONS: u32 = 10;
/// Dollar price one coupon pays for an item of up to (`BankHelperServer.PagarCompra`).
pub const COUPON_VALUE: i64 = 145;

/// Order-book side of an offer (`HelperOffer`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Buy,
    Sell,
}

impl Side {
    /// Returns the side that trades against this one.
    #[must_use]
    pub const fn other(self) -> Self {
        match self {
            Self::Buy => Self::Sell,
            Self::Sell => Self::Buy,
        }
    }

    /// Whether an order on this side at `price` accepts an opposing offer at `offered_price`.
    #[must_use]
    pub fn accepts(self, offered_price: f64, price: f64) -> bool {
        match self {
            Self::Buy => offered_price <= price,
            Self::Sell => offered_price >= price,
        }
    }
}

/// Returns the fee of a transfer of gross `amount` dollars (`TransactionsPanel.GetFeeAmount`).
#[must_use]
pub const fn transfer_fee(amount: i64) -> i64 {
    amount * TRANSFER_FEE_PERCENT / 100
}

/// Returns the coins a coin round pays: the `reward`, limited to what is left of `max_supply` after
/// `in_supply` coins (`BlockchainSystem.CheckBlocks`).
#[must_use]
pub fn round_reward(reward: f64, max_supply: f64, in_supply: f64) -> f64 {
    reward.min(max_supply - in_supply)
}

/// Returns the rounded average trade price from coin `amount` and total `value`, or 0 without
/// trades (`Coin.GetPendingTradeValue`).
#[must_use]
pub const fn pending_trade_price(value: i64, amount: i64) -> i64 {
    if amount <= 0 || value <= 0 {
        return 0;
    }

    (value + amount / 2) / amount
}

/// Returns the midpoint of the best buy and sell prices, or the price of the side that has offers
/// (`Coin.GetOrderBookValue`); a side without offers has price 0.
#[must_use]
#[expect(
    clippy::cast_precision_loss,
    reason = "Coin.GetOrderBookValue returns integer prices through the script's f64 representation"
)]
pub fn order_book_price(best_buy: i64, best_sell: i64) -> f64 {
    match (best_buy, best_sell) {
        (buy, sell) if buy > 0 && sell > 0 => ((buy + sell + 1) / 2) as f64,
        (_, sell) if sell > 0 => sell as f64,
        (buy, _) => buy as f64,
    }
}

/// Returns the coupons that pay a shop item of `price` dollars (`BankHelperServer.PagarCompra`).
#[must_use]
pub const fn coupon_cost(price: i64) -> i64 {
    let cost = (price + COUPON_VALUE - 1) / COUPON_VALUE;

    if cost < 1 { 1 } else { cost }
}

/// Whether a monthly payment whose amount became `amount` after money was added to it or taken off
/// it by a rental ends (`BankSubs.AddPayment`, `BankSubs.RemoveRentalPayments`).
#[must_use]
pub const fn payment_ends(amount: i64) -> bool {
    amount == 0
}

/// Whether the monthly administrator payment ends once a hired administrator's charge came off it
/// (`BankSubs.RemoveAdminPayment`), which also ends a payment that went below zero.
#[must_use]
pub const fn admin_payment_ends(amount: i64) -> bool {
    amount <= 0
}

/// Converts a mining cycle of `hours` to world seconds, with a one-second minimum.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "Coin.SetCoinRate uses DateTime.AddHours and the cycle is stored as whole world seconds"
)]
pub fn cycle_seconds(hours: f64) -> u64 {
    (hours * SECONDS_PER_HOUR as f64).round().max(1.0) as u64
}

/// Hash power of a CPU core per GHz (`BlockchainSystem.CheckBlocks`).
const CPU_POWER_PER_GHZ_CORE: f32 = 0.15;
/// Hash power of a RAM megabyte per RAM model (`BlockchainSystem.CheckBlocks`).
const RAM_POWER_PER_MB_MODEL: f32 = 0.0005;

/// Returns the hash power a machine adds to a mining round (`BlockchainSystem.CheckBlocks`): per
/// CPU `(speed, cores)`, speed × cores × 0.15; per RAM `(memory_mb, model)`, memory × 0.0005 ×
/// model; plus the hashrate of each GPU.
#[must_use]
#[expect(
    clippy::cast_precision_loss,
    reason = "BlockchainSystem.CheckBlocks performs hardware power arithmetic with C# floats"
)]
pub fn hash_power(
    cpus: impl IntoIterator<Item = (f32, u32)>,
    rams: impl IntoIterator<Item = (u32, u32)>,
    gpus: impl IntoIterator<Item = f32>,
) -> f32 {
    let cpus: f32 = cpus
        .into_iter()
        .map(|(speed, cores)| speed * cores as f32 * CPU_POWER_PER_GHZ_CORE)
        .sum();
    let rams: f32 = rams
        .into_iter()
        .map(|(memory_mb, model)| memory_mb as f32 * RAM_POWER_PER_MB_MODEL * model as f32)
        .sum();
    let gpus: f32 = gpus.into_iter().sum();

    cpus + rams + gpus
}

/// Picks the wallet that wins a mining round, weighting each by its total mining power in `powers`
/// (`BlockchainSystem.CheckBlocks`). Draws once even when there is no wallet.
#[expect(
    clippy::cast_precision_loss,
    reason = "BlockchainSystem.CheckBlocks scales its random draw with C# double arithmetic"
)]
pub fn pick_by_power(rng: &mut Rng, powers: &[f64]) -> Option<usize> {
    let total: f64 = powers.iter().sum();
    let mut draw = (rng.next_u64() as f64 / u64::MAX as f64) * total;
    let mut choice = powers.len().checked_sub(1)?;

    for (index, power) in powers.iter().enumerate() {
        if draw <= *power {
            choice = index;
            break;
        }

        draw -= power;
    }

    Some(choice)
}

/// Picks one of the `machines` of the winning wallet uniformly (`BlockchainSystem.CheckBlocks`).
pub fn pick_machine(rng: &mut Rng, machines: usize) -> usize {
    usize::try_from(rng.next_u64() % machines as u64).unwrap_or(0)
}
