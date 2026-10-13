use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::coin_id;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::CoinGetMinedCoins;

#[native(method)]
pub fn get_mined_coins(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(id) = coin_id(vm, this) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = CoinGetMinedCoins {
        program: program(vm)?,
        coin: id,
    };

    Ok(respond(
        vm,
        server(vm)?.send_coin_get_mined_coins(&message),
        |_, amount| amount,
    ))
}
