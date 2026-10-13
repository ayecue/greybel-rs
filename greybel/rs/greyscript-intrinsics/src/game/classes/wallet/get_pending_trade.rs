use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::wallet_id;
use crate::error::IntrinsicError;
use crate::game::client::failure::failure_value;
use crate::game::client::server::{program, respond, server};
use crate::protocol::{PendingTrade, Side, WalletGetPendingTrade};

/// Returns the wallet's open `coinName` order as `[side, amount, price]`, or Grey Hack's error text.
#[native(method)]
pub fn get_pending_trade(
    vm: &mut Vm,
    this: &Value,
    #[name = "coinName"] coin: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let (Some(id), Some(coin)) = (wallet_id(vm, this), coin.as_str()) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = WalletGetPendingTrade {
        program: program(vm)?,
        wallet: id,
        coin: coin.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_wallet_get_pending_trade(&message),
        |vm, trade| match trade {
            PendingTrade::Null => Ok::<_, IntrinsicError>(Value::Null),
            PendingTrade::Failure(failure) => {
                failure_value("get_pending_trade", &failure)
            }
            PendingTrade::Order {
                side,
                amount,
                price,
            } => {
                let side = match side {
                    Side::Buy => "Buy",
                    Side::Sell => "Sell",
                };
                let values = vec![Value::from(side), Value::from(amount), Value::from(price)];

                Ok(Value::List(vm.heap_mut().add_list(values)))
            }
        },
    ))
}
