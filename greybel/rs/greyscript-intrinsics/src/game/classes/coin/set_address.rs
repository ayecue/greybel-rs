use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::coin_id;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::CoinSetAddress;

#[native(method)]
pub fn set_address(
    vm: &mut Vm,
    this: &Value,
    address: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let Some(address) = address.as_str() else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let Some(id) = coin_id(vm, this) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = CoinSetAddress {
        program: program(vm)?,
        coin: id,
        address: address.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_coin_set_address(&message),
        |_, result| result,
    ))
}
