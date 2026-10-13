use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::blockchain;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::BlockchainGetCoinName;

/// Returns the name of the script user's coin when `user`/`password` authenticate its creator.
#[native(method)]
pub fn get_coin_name(
    vm: &mut Vm,
    this: &Value,
    user: &Value,
    password: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let Some(blockchain) = blockchain(vm, this) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = BlockchainGetCoinName {
        program: program(vm)?,
        blockchain,
        user: user.as_str().map(Into::into),
        password: password.as_str().map(Into::into),
    };

    Ok(respond(
        vm,
        server(vm)?.send_blockchain_get_coin_name(&message),
        |_, name| name.response("get_coin_name"),
    ))
}
