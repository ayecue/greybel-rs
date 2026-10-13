use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::sub_wallet;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::SubWalletCheckPassword;

#[native(method)]
pub fn check_password(
    vm: &mut Vm,
    this: &Value,
    password: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let (Some(password), Some(sub_wallet)) = (password.as_str(), sub_wallet(vm, this)) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = SubWalletCheckPassword {
        program: program(vm)?,
        sub_wallet,
        password: password.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_sub_wallet_check_password(&message),
        |_, matches| matches,
    ))
}
