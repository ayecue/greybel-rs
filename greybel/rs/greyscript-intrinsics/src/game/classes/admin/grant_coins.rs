use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::is_admin;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::AdminGrantCoins;

/// Grants wallet `walletUser` a balance of `amount` in `coin`; not available in Grey Hack.
#[native(method)]
pub fn grant_coins(
    vm: &mut Vm,
    this: &Value,
    #[name = "walletUser"] wallet_user: &Value,
    coin: &Value,
    amount: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    if !is_admin(vm, this, "grant_coins") {
        return Ok(NativeOutput::ready(Value::Null));
    }

    let (Some(wallet_user), Some(coin), Some(amount)) =
        (wallet_user.as_str(), coin.as_str(), amount.as_number())
    else {
        return Err(IntrinsicError::InvalidArguments("grant_coins"));
    };
    let message = AdminGrantCoins {
        program: program(vm)?,
        wallet_user: wallet_user.into(),
        coin: coin.into(),
        amount,
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_grant_coins(&message),
        |_, granted: bool| granted,
    ))
}
