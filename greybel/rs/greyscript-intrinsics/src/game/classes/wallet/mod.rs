//! `wallet` balance, PIN, and market intrinsics.

use std::ops::ControlFlow;

use greybel_interpreter::{Native, Value, Vm};
use greyhack_behaviour::check_wallet_password;

use crate::error::IntrinsicError;
use crate::game::client::check::{Proceed, stop};
use crate::game::client::object::{GameObject, Kind};
use crate::game::client::receiver::receiver;
use crate::game::client::server::refuse;
use crate::game::client::{object, receiver};
use crate::protocol::ObjectId;

mod buy_coin;
mod cancel_pending_trade;
mod get_balance;
mod get_global_offers;
mod get_pending_trade;
mod get_pin;
mod list_coins;
mod list_global_coins;
mod reset_password;
mod sell_coin;
mod show_nodes;

pub(crate) const METHODS: &[(&str, &Native)] = &[
    ("list_coins", &list_coins::list_coins::NATIVE),
    ("get_balance", &get_balance::get_balance::NATIVE),
    (
        "list_global_coins",
        &list_global_coins::list_global_coins::NATIVE,
    ),
    ("show_nodes", &show_nodes::show_nodes::NATIVE),
    ("get_pin", &get_pin::get_pin::NATIVE),
    ("buy_coin", &buy_coin::buy_coin::NATIVE),
    ("sell_coin", &sell_coin::sell_coin::NATIVE),
    (
        "get_pending_trade",
        &get_pending_trade::get_pending_trade::NATIVE,
    ),
    (
        "cancel_pending_trade",
        &cancel_pending_trade::cancel_pending_trade::NATIVE,
    ),
    (
        "get_global_offers",
        &get_global_offers::get_global_offers::NATIVE,
    ),
    ("reset_password", &reset_password::reset_password::NATIVE),
];

/// Constructs the wallet object for server-issued `id`.
pub(crate) fn create(vm: &mut Vm, id: ObjectId) -> Result<Value, IntrinsicError> {
    object::create_of(vm, Kind::Wallet, GameObject::Wallet, id)
}

/// Returns the server identity of wallet object `this`.
fn wallet_id(vm: &Vm, this: &Value) -> Option<ObjectId> {
    receiver::<receiver::Wallet>(vm, this).map(|wallet| wallet.reference())
}

/// Order arguments of `buy_coin` and `sell_coin`, with amount and price truncated to integers.
struct Order<'a> {
    wallet: ObjectId,
    coin: &'a str,
    amount: f64,
    price: f64,
    subwallet: &'a str,
}

/// Runs the checks `WalletIntrinsics.buy_coin` and `sell_coin` share, in the game's order.
fn order<'a>(
    vm: &Vm,
    this: &Value,
    intrinsic: &'static str,
    coin: &'a Value,
    amount: &Value,
    price: &Value,
    subwallet: &'a Value,
) -> Result<Proceed<Order<'a>>, IntrinsicError> {
    if !object::game_map(vm, this) {
        return Err(IntrinsicError::InvalidArguments(intrinsic));
    }

    let (Some(amount), Some(coin), Some(price), Some(subwallet)) = (
        amount.as_number(),
        coin.as_str(),
        price.as_number(),
        subwallet.as_str(),
    ) else {
        return Err(IntrinsicError::InvalidArguments(intrinsic));
    };
    let Some(wallet) = wallet_id(vm, this) else {
        return Ok(stop(Value::Null));
    };

    if amount.trunc() < 1.0 || price.trunc() < 1.0 {
        return Ok(stop(
            "sell_coin: coin amount and proposed price cannot be 0.",
        ));
    }

    Ok(ControlFlow::Continue(Order {
        wallet,
        coin,
        amount: amount.trunc(),
        price: price.trunc(),
        subwallet,
    }))
}

/// Reads the new password and runs the rule checks `WalletIntrinsics.reset_password` and `CoinIntrinsics.reset_password` share.
pub(super) fn reset_password_argument(password: &Value) -> Result<Proceed<&str>, IntrinsicError> {
    let Some(password) = password.as_str() else {
        return Ok(stop(Value::Null));
    };

    if let Err(refusal) = check_wallet_password(password) {
        return refuse("reset_password", refusal).map(ControlFlow::Break);
    }

    Ok(ControlFlow::Continue(password))
}
