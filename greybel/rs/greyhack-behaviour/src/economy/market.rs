//! Market-order limits, sub-wallet balance rules, and bank subscription types.

use crate::base::refusal::{Refusal, RuleResult};
use crate::economy::coin_limits::MAX_COINS;

enum_def! {
    /// Recurring bank payment type (`BankSubs.PaymentID`).
    pub enum SubscriptionKind {
        Bank = 0,
        Rented = 1,
        Admin = 2,
    }
}

/// Monthly price of a hired rental administrator (`ConfigLanUI.Precios.ADMIN`).
pub const RENTAL_ADMIN_PRICE: i64 = 2500;
/// Monthly price of each computer of a rented network (`ConfigLanUI.Precios.COMPUTER`).
pub const RENTAL_COMPUTER_PRICE: i64 = 75;
/// Monthly price of each router of a rented network (`ConfigLanUI.Precios.ROUTER`).
pub const RENTAL_ROUTER_PRICE: i64 = 50;
/// Monthly price of each hub of a rented network (`ConfigLanUI.Precios.HUBS`).
pub const RENTAL_HUB_PRICE: i64 = 35;
/// Monthly price of each switch of a rented network (`ConfigLanUI.Precios.SWITCH`).
pub const RENTAL_SWITCH_PRICE: i64 = 40;

/// What a monthly payment does to the networks its subscription lists
/// (`BankSubs.Payment.ProcessPayment`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkEffect {
    /// The listed networks stay as they are.
    Skip,
    /// The listed networks are enabled (`BankSubs.Payment.DisableNetwork` with `disable: false`).
    Enable,
    /// The listed networks are disabled for nonpayment.
    Disable,
}

impl SubscriptionKind {
    /// Returns what a payment of this kind that `paid` or not does to its networks
    /// (`BankSubs.Payment.ProcessPayment`). Bank fees never touch networks. A rental or an admin
    /// charge that is paid enables its networks; a rental charge that is not paid disables them.
    /// The unpaid admin charge would remove the admin of each listed network, but the game never
    /// lists a network on an admin subscription (`BankSubs.AddPayment` is called without an
    /// address for it), so it changes nothing.
    #[must_use]
    pub const fn network_effect(self, paid: bool) -> NetworkEffect {
        match (self, paid) {
            (Self::Bank, _) | (Self::Admin, false) => NetworkEffect::Skip,
            (Self::Rented | Self::Admin, true) => NetworkEffect::Enable,
            (Self::Rented, false) => NetworkEffect::Disable,
        }
    }
}

/// The devices of a rented network that are paid for (`NetworkLan.GetTotalComputers`,
/// `NetworkLan.GetTotalDevices`): every device that is not the player's own computer, and every router
/// including the main router.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RentalDeviceTable {
    pub computers: i64,
    pub routers: i64,
    pub hubs: i64,
    pub switches: i64,
}

/// Returns the monthly price of an ISP speed pack (`ISPPanel.PackPrice`). A speed that is not a pack has
/// the number itself as its price, as parsing the speed's number as a pack price does.
#[must_use]
pub fn net_speed_price(net_speed: u8) -> i64 {
    match net_speed {
        3 => 10,
        6 => 25,
        12 => 40,
        20 => 60,
        25 => 170,
        other => i64::from(other),
    }
}

/// Returns the monthly price of a rented network (`ConfigLanUI.CalculaPrecio` for a network with no
/// pending changes): its devices, the hired administrator and the speed pack.
#[must_use]
pub fn rental_price(devices: RentalDeviceTable, admin_hired: bool, net_speed: u8) -> i64 {
    devices.computers * RENTAL_COMPUTER_PRICE
        + devices.routers * RENTAL_ROUTER_PRICE
        + devices.hubs * RENTAL_HUB_PRICE
        + devices.switches * RENTAL_SWITCH_PRICE
        + if admin_hired { RENTAL_ADMIN_PRICE } else { 0 }
        + net_speed_price(net_speed)
}

/// Returns what `price`, the monthly price of a rented network, is without the hired administrator
/// when `admin_hired`, as the payments of a rented network that is removed are
/// (`BankSubs.RemoveRentalPayments`).
#[must_use]
pub const fn rental_price_without_admin(price: i64, admin_hired: bool) -> i64 {
    if admin_hired {
        price - RENTAL_ADMIN_PRICE
    } else {
        price
    }
}

/// Maximum coin amount per order.
pub const MAX_ORDER_COINS: f64 = 50_000.0;
/// Maximum unit price per order.
pub const MAX_ORDER_PRICE: f64 = 10_000.0;
/// Minimum explicit sub-wallet transfer amount.
pub const MIN_TRANSFER_AMOUNT: i64 = 1;
/// Maximum coin balance a sub-wallet may contain ([`MAX_COINS`]).
pub const MAX_BALANCE: f64 = MAX_COINS as f64;

/// Validates order amount and unit price for `buy_coin` and `sell_coin`, without consulting wallets or the order book.
pub fn check_order(amount: f64, price: f64) -> RuleResult {
    if amount <= 0.0 || price <= 0.0 {
        Err(Refusal::EmptyOrder)
    } else if amount > MAX_ORDER_COINS {
        Err(Refusal::TooManyOrderCoins)
    } else if price > MAX_ORDER_PRICE {
        Err(Refusal::OrderPriceTooHigh)
    } else {
        Ok(())
    }
}

/// Checks that an explicit transfer meets the minimum amount.
#[must_use]
pub const fn valid_transfer_amount(amount: i64) -> bool {
    amount >= MIN_TRANSFER_AMOUNT
}

/// Clamps a balance to `SubWallet.AddAmount`'s minimum and maximum.
#[must_use]
pub fn clamp_balance(balance: f64) -> f64 {
    balance.clamp(0.0, MAX_BALANCE)
}

/// Limits the balance after a mining reward to the sub-wallet maximum.
#[must_use]
pub fn cap_balance(balance: f64) -> f64 {
    balance.min(MAX_BALANCE)
}
