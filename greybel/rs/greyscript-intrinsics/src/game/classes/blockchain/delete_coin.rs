use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::blockchain;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::BlockchainDeleteCoin;

#[native(method)]
pub fn delete_coin(
    vm: &mut Vm,
    this: &Value,
    #[name = "coinName"] name: &Value,
    user: &Value,
    password: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let Some(blockchain) = blockchain(vm, this) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = BlockchainDeleteCoin {
        program: program(vm)?,
        blockchain,
        coin: name.as_str().map(Into::into),
        user: user.as_str().map(Into::into),
        password: password.as_str().map(Into::into),
    };

    Ok(respond(
        vm,
        server(vm)?.send_blockchain_delete_coin(&message),
        |_, outcome| outcome.response("delete_coin"),
    ))
}
