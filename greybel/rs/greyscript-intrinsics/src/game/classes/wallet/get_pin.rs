use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::wallet_id;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::WalletGetPin;

/// Returns the wallet PIN.
#[native(method)]
pub fn get_pin(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(id) = wallet_id(vm, this) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = WalletGetPin {
        program: program(vm)?,
        wallet: id,
    };

    Ok(respond(
        vm,
        server(vm)?.send_wallet_get_pin(&message),
        |_, pin| pin,
    ))
}
