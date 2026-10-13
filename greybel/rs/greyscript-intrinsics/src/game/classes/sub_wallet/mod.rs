//! `subwallet` queries and info-note updates (`SubWalletIntrinsics`), one file per method.

use greybel_interpreter::{Native, Value, Vm};

use crate::error::IntrinsicError;
use crate::game::client::object::{GameObject, Kind};
use crate::game::client::receiver::receiver;
use crate::game::client::{object, receiver};
use crate::protocol::ObjectId;

mod check_password;
mod delete_subwallet;
mod get_balance_subwallet;
mod get_info;
mod get_user;
mod last_transaction;
mod mining;
mod set_info;
mod wallet_username;

pub(crate) const METHODS: &[(&str, &Native)] = &[
    (
        "get_balance",
        &get_balance_subwallet::get_balance_subwallet::NATIVE,
    ),
    ("get_user", &get_user::get_user::NATIVE),
    ("check_password", &check_password::check_password::NATIVE),
    ("get_info", &get_info::get_info::NATIVE),
    ("set_info", &set_info::set_info::NATIVE),
    ("wallet_username", &wallet_username::wallet_username::NATIVE),
    (
        "last_transaction",
        &last_transaction::last_transaction::NATIVE,
    ),
    ("mining", &mining::mining::NATIVE),
    ("delete", &delete_subwallet::delete_subwallet::NATIVE),
];

/// Constructs the sub-wallet object for server-issued `id`.
pub(crate) fn create(vm: &mut Vm, id: ObjectId) -> Result<Value, IntrinsicError> {
    object::create_of(vm, Kind::SubWallet, GameObject::SubWallet, id)
}

/// Returns the server identity of sub-wallet object `this`.
fn sub_wallet(vm: &Vm, this: &Value) -> Option<ObjectId> {
    receiver::<receiver::SubWallet>(vm, this).map(|sub_wallet| sub_wallet.reference())
}
