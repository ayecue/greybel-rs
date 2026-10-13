//! `blockchain` coin queries and wallet intrinsics (`BlockchainIntrinsics`), one file per method.
//! Every method first requires the library's host computer to be online.

use greybel_interpreter::{Native, Value, Vm};

use crate::error::IntrinsicError;
use crate::game::classes::wallet;
use crate::game::client::failure::failure_value;
use crate::game::client::receiver;
use crate::game::client::receiver::receiver;
use crate::protocol::{Figure, ObjectId, WalletLogin};

mod mine_amount;
mod coin_price;
mod create_wallet;
mod delete_coin;
mod get_coin;
mod get_coin_name;
mod login_wallet;
mod show_history;

pub(crate) const METHODS: &[(&str, &Native)] = &[
    ("coin_price", &coin_price::coin_price::NATIVE),
    ("show_history", &show_history::show_history::NATIVE),
    ("amount_mined", &mine_amount::amount_mined::NATIVE),
    ("get_coin", &get_coin::get_coin::NATIVE),
    ("create_wallet", &create_wallet::create_wallet::NATIVE),
    ("login_wallet", &login_wallet::login_wallet::NATIVE),
    ("delete_coin", &delete_coin::delete_coin::NATIVE),
    ("get_coin_name", &get_coin_name::get_coin_name::NATIVE),
];

/// Returns the server identity of blockchain object `this`.
fn blockchain(vm: &Vm, this: &Value) -> Option<ObjectId> {
    receiver::<receiver::Blockchain>(vm, this).map(|access| access.reference())
}

/// Converts a `figure` response into the script value returned by `intrinsic`.
pub(crate) fn figure(intrinsic: &'static str, figure: Figure) -> Result<Value, IntrinsicError> {
    Ok(match figure {
        Figure::Null => Value::Null,
        Figure::Number(number) => Value::from(number),
        Figure::Failure(failure) => failure_value(intrinsic, &failure)?,
    })
}

/// Converts a `login` response into `intrinsic`'s script value, constructing any opened wallet object.
fn wallet_value(
    vm: &mut Vm,
    intrinsic: &'static str,
    login: WalletLogin,
) -> Result<Value, IntrinsicError> {
    match login {
        WalletLogin::Null => Ok(Value::Null),
        WalletLogin::Wallet(id) => wallet::create(vm, id),
        WalletLogin::Failure(failure) => failure_value(intrinsic, &failure),
    }
}
