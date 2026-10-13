use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::{blockchain, wallet_value};
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::BlockchainLoginWallet;

/// Opens the wallet authenticated by `user`/`password`.
#[native(method)]
pub fn login_wallet(
    vm: &mut Vm,
    this: &Value,
    user: &Value,
    password: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let Some(blockchain) = blockchain(vm, this) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = BlockchainLoginWallet {
        program: program(vm)?,
        blockchain,
        user: user.as_str().map(Into::into),
        password: password.as_str().map(Into::into),
    };

    Ok(respond(
        vm,
        server(vm)?.send_blockchain_login_wallet(&message),
        |vm, login| wallet_value(vm, "login_wallet", login),
    ))
}
