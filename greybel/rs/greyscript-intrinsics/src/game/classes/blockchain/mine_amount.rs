use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::{blockchain, figure};
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::BlockchainAmountMined;

#[native(method)]
pub fn amount_mined(
    vm: &mut Vm,
    this: &Value,
    #[name = "coinName"] name: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let (Some(name), Some(blockchain)) = (name.as_str(), blockchain(vm, this)) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = BlockchainAmountMined {
        program: program(vm)?,
        blockchain,
        coin: name.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_blockchain_amount_mined(&message),
        |_, amount| figure("amount_mined", amount),
    ))
}
