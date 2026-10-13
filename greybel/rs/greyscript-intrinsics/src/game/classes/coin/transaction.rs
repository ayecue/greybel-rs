use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::coin_id;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::CoinTransaction;

#[native(method)]
pub fn transaction(
    vm: &mut Vm,
    this: &Value,
    #[name = "subWalletOrig"] origin: &Value,
    #[name = "subWalletDest"] destination: &Value,
    #[name = "valAmount"] amount: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let Some(id) = coin_id(vm, this) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let (Some(origin), Some(destination), Some(amount)) =
        (origin.as_str(), destination.as_str(), amount.as_int())
    else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = CoinTransaction {
        program: program(vm)?,
        coin: id,
        origin: origin.into(),
        destination: destination.into(),
        amount: i64::from(amount),
    };

    Ok(respond(
        vm,
        server(vm)?.send_coin_transaction(&message),
        |_, outcome| outcome.response("transaction"),
    ))
}
