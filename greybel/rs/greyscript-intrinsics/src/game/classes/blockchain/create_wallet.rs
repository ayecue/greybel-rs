use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::{blockchain, wallet_value};
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::BlockchainCreateWallet;

/// Creates a wallet for `user`/`password`; requires the player to have a bank account.
#[native(method)]
pub fn create_wallet(
    vm: &mut Vm,
    this: &Value,
    user: &Value,
    password: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let Some(blockchain) = blockchain(vm, this) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = BlockchainCreateWallet {
        program: program(vm)?,
        blockchain,
        user: user.as_str().map(Into::into),
        password: password.as_str().map(Into::into),
    };

    Ok(respond(
        vm,
        server(vm)?.send_blockchain_create_wallet(&message),
        |vm, login| wallet_value(vm, "create_wallet", login),
    ))
}
