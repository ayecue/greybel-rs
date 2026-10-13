use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::sub_wallet;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::SubWalletWalletUsername;

#[native(method)]
pub fn wallet_username(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(sub_wallet) = sub_wallet(vm, this) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = SubWalletWalletUsername {
        program: program(vm)?,
        sub_wallet,
    };

    Ok(respond(
        vm,
        server(vm)?.send_sub_wallet_wallet_username(&message),
        |_, user| user,
    ))
}
