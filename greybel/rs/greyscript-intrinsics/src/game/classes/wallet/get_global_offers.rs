use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::wallet_id;
use crate::error::IntrinsicError;
use crate::game::client::failure::failure_value;
use crate::game::client::server::{program, respond, server};
use crate::protocol::{GlobalOffers, Side, WalletGetGlobalOffers};

/// Returns all standing `coinName` offers keyed by wallet user, each as `[side, amount, price]`.
#[native(method)]
pub fn get_global_offers(
    vm: &mut Vm,
    this: &Value,
    #[name = "coinName"] coin: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let (Some(id), Some(coin)) = (wallet_id(vm, this), coin.as_str()) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = WalletGetGlobalOffers {
        program: program(vm)?,
        wallet: id,
        coin: coin.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_wallet_get_global_offers(&message),
        |vm, offers| match offers {
            GlobalOffers::Failure(failure) => failure_value("get_global_offers", &failure),
            GlobalOffers::Offers(offers) => {
                let heap = vm.heap_mut();
                let entries: Vec<(Value, Value)> = offers
                    .into_iter()
                    .map(|offer| {
                        let side = match offer.side {
                            Side::Buy => "Buy",
                            Side::Sell => "Sell",
                        };
                        let order = heap.add_list(vec![
                            Value::from(side),
                            Value::from(offer.amount),
                            Value::from(offer.price),
                        ]);

                        (Value::from(offer.wallet_user), Value::List(order))
                    })
                    .collect();

                Ok(Value::Map(heap.add_map_from(entries)))
            }
        },
    ))
}
