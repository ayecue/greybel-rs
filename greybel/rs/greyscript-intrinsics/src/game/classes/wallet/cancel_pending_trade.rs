use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::wallet_id;
use crate::error::IntrinsicError;
use crate::game::client::failure::failure_value;
use crate::game::client::server::{program, respond, server};
use crate::protocol::{CancelTrade, WalletCancelPendingTrade};

/// Cancels the wallet's open order for `coinName`.
#[native(method)]
pub fn cancel_pending_trade(
    vm: &mut Vm,
    this: &Value,
    #[name = "coinName"] coin: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let (Some(id), Some(coin)) = (wallet_id(vm, this), coin.as_str()) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = WalletCancelPendingTrade {
        program: program(vm)?,
        wallet: id,
        coin: coin.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_wallet_cancel_pending_trade(&message),
        |_, outcome| match outcome {
            CancelTrade::Null => Ok::<_, IntrinsicError>(Value::Null),
            CancelTrade::Success => Ok(Value::from("")),
            CancelTrade::Failure(failure) => {
                failure_value("cancel_pending_trade", &failure)
            }
        },
    ))
}
