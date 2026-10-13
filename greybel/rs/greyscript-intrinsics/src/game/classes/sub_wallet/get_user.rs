use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::sub_wallet;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::SubWalletGetUser;

#[native(method)]
pub fn get_user(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(sub_wallet) = sub_wallet(vm, this) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = SubWalletGetUser {
        program: program(vm)?,
        sub_wallet,
    };

    Ok(respond(
        vm,
        server(vm)?.send_sub_wallet_get_user(&message),
        |_, user| user,
    ))
}
