use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::{reset_password_argument, wallet_id};
use crate::error::IntrinsicError;
use crate::game::client::check::proceed;
use crate::game::client::server::{program, respond, server};
use crate::protocol::WalletResetPassword;

#[native(method)]
pub fn reset_password(
    vm: &mut Vm,
    this: &Value,
    #[name = "newPassword"] new_password: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let Some(id) = wallet_id(vm, this) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let password = proceed!(reset_password_argument(new_password));

    let message = WalletResetPassword {
        program: program(vm)?,
        wallet: id,
        password: password.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_wallet_reset_password(&message),
        |_, outcome| outcome.response("reset_password"),
    ))
}
