//! `coin` address, mining-configuration, and sub-wallet intrinsics (`CoinIntrinsics`), one file per method.

use greybel_interpreter::{Native, Value, Vm};

use crate::error::IntrinsicError;
use crate::game::client::object::{GameObject, Kind};
use crate::game::client::receiver::receiver;
use crate::game::client::{object, receiver};
use crate::protocol::ObjectId;

mod create_subwallet;
mod get_address;
mod get_cycle_mining;
mod get_mined_coins;
mod get_reward;
mod get_subwallet;
mod get_subwallets;
mod reset_password;
mod set_address;
mod set_cycle_mining;
mod set_reward;
mod transaction;

pub(crate) const METHODS: &[(&str, &Native)] = &[
    ("get_address", &get_address::get_address::NATIVE),
    ("set_address", &set_address::set_address::NATIVE),
    ("get_mined_coins", &get_mined_coins::get_mined_coins::NATIVE),
    ("get_reward", &get_reward::get_reward::NATIVE),
    ("set_reward", &set_reward::set_reward::NATIVE),
    (
        "get_cycle_mining",
        &get_cycle_mining::get_cycle_mining::NATIVE,
    ),
    (
        "set_cycle_mining",
        &set_cycle_mining::set_cycle_mining::NATIVE,
    ),
    (
        "create_subwallet",
        &create_subwallet::create_subwallet::NATIVE,
    ),
    ("get_subwallet", &get_subwallet::get_subwallet::NATIVE),
    ("get_subwallets", &get_subwallets::get_subwallets::NATIVE),
    ("transaction", &transaction::transaction::NATIVE),
    (
        "reset_password",
        &reset_password::reset_password_coin::NATIVE,
    ),
];

/// Constructs the coin object for server-issued `id`.
pub(crate) fn create(vm: &mut Vm, id: ObjectId) -> Result<Value, IntrinsicError> {
    object::create_of(vm, Kind::Coin, GameObject::Coin, id)
}

/// Returns the server identity of coin object `this`.
fn coin_id(vm: &Vm, this: &Value) -> Option<ObjectId> {
    receiver::<receiver::Coin>(vm, this).map(|coin| coin.reference())
}
