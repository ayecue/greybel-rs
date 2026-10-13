use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::coin_id;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::CoinGetCycleMining;

#[native(method)]
pub fn get_cycle_mining(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(id) = coin_id(vm, this) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = CoinGetCycleMining {
        program: program(vm)?,
        coin: id,
    };

    Ok(respond(
        vm,
        server(vm)?.send_coin_get_cycle_mining(&message),
        |_, cycle| cycle,
    ))
}
