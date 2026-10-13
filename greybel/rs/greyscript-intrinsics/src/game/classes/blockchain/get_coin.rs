use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::blockchain;
use crate::error::IntrinsicError;
use crate::game::classes::coin;
use crate::game::client::failure::failure_value;
use crate::game::client::server::{program, respond, server};
use crate::protocol::{BlockchainGetCoin, CoinLogin};

/// Returns coin `coinName` when `user`/`password` authenticate its creator.
#[native(method)]
pub fn get_coin(
    vm: &mut Vm,
    this: &Value,
    #[name = "coinName"] name: &Value,
    user: &Value,
    password: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let Some(blockchain) = blockchain(vm, this) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = BlockchainGetCoin {
        program: program(vm)?,
        blockchain,
        user: user.as_str().map(Into::into),
        password: password.as_str().map(Into::into),
        coin: name.as_str().map(Into::into),
    };

    Ok(respond(
        vm,
        server(vm)?.send_blockchain_get_coin(&message),
        |vm, login| match login {
            CoinLogin::Null => Ok(Value::Null),
            CoinLogin::Coin(id) => coin::create(vm, id),
            CoinLogin::Failure(failure) => failure_value("get_coin", &failure),
        },
    ))
}
