use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::wallet_id;
use crate::error::IntrinsicError;
use crate::game::classes::blockchain::figure;
use crate::game::client::server::{program, respond, server};
use crate::protocol::WalletGetBalance;

/// Returns the balance in `coinName`, or null when `coinName` is omitted.
#[native(method)]
pub fn get_balance(
    vm: &mut Vm,
    this: &Value,
    #[name = "coinName"] coin: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let (Some(id), Some(coin)) = (wallet_id(vm, this), coin.as_str()) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = WalletGetBalance {
        program: program(vm)?,
        wallet: id,
        coin: coin.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_wallet_get_balance(&message),
        |_, balance| figure("get_balance", balance),
    ))
}
