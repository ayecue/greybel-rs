use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::{blockchain, figure};
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::BlockchainCoinPrice;

#[native(method)]
pub fn coin_price(
    vm: &mut Vm,
    this: &Value,
    #[name = "coinName"] name: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let (Some(name), Some(blockchain)) = (name.as_str(), blockchain(vm, this)) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = BlockchainCoinPrice {
        program: program(vm)?,
        blockchain,
        coin: name.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_blockchain_coin_price(&message),
        |_, price| figure("coin_price", price),
    ))
}
