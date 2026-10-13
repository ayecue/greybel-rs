use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::coin_id;
use crate::game::classes::wallet::reset_password_argument;
use crate::error::IntrinsicError;
use crate::game::client::check::proceed;
use crate::game::client::server::{program, respond, server};
use crate::protocol::CoinResetPasswordCoin;

#[native(method)]
pub fn reset_password_coin(
    vm: &mut Vm,
    this: &Value,
    #[name = "newPassword"] new_password: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let Some(id) = coin_id(vm, this) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let password = proceed!(reset_password_argument(new_password));

    let message = CoinResetPasswordCoin {
        program: program(vm)?,
        coin: id,
        password: password.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_coin_reset_password_coin(&message),
        |_, outcome| outcome.response("reset_password"),
    ))
}
