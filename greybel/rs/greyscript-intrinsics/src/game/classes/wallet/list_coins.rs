use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::wallet_id;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::WalletListCoins;

/// Returns coins in which the wallet holds a balance.
#[native(method)]
pub fn list_coins(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(id) = wallet_id(vm, this) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = WalletListCoins {
        program: program(vm)?,
        wallet: id,
    };

    Ok(respond(
        vm,
        server(vm)?.send_wallet_list_coins(&message),
        |_, coins| coins,
    ))
}
