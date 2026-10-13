use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::wallet_id;
use crate::error::IntrinsicError;
use crate::game::classes::blockchain::figure;
use crate::game::client::server::{program, respond, server};
use crate::protocol::WalletShowNodes;

/// Returns the number of pending mining jobs for a wallet and coin.
#[native(method)]
pub fn show_nodes(
    vm: &mut Vm,
    this: &Value,
    #[name = "coinName"] coin: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let (Some(id), Some(coin)) = (wallet_id(vm, this), coin.as_str()) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = WalletShowNodes {
        program: program(vm)?,
        wallet: id,
        coin: coin.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_wallet_show_nodes(&message),
        |_, nodes| figure("show_nodes", nodes),
    ))
}
