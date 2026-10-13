use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::coin_id;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::CoinSetCycleMining;

#[native(method)]
pub fn set_cycle_mining(
    vm: &mut Vm,
    this: &Value,
    #[name = "rateHours"]
    #[default(3)]
    cycle: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let (Some(cycle), Some(id)) = (cycle.as_number(), coin_id(vm, this)) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = CoinSetCycleMining {
        program: program(vm)?,
        coin: id,
        cycle_mining: cycle,
    };

    Ok(respond(
        vm,
        server(vm)?.send_coin_set_cycle_mining(&message),
        |_, result| result,
    ))
}
