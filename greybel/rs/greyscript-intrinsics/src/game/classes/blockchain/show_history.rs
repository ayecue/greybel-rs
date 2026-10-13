use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::blockchain;
use crate::error::IntrinsicError;
use crate::game::client::failure::failure_value;
use crate::game::client::server::{program, respond, server};
use crate::protocol::{BlockchainShowHistory, PriceHistory};

/// Returns `BlockchainSystem.GetHistory` as `{index: [price, date]}`, oldest first.
/// Unknown coins return `null`.
#[native(method)]
pub fn show_history(
    vm: &mut Vm,
    this: &Value,
    #[name = "coinName"] name: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let (Some(name), Some(blockchain)) = (name.as_str(), blockchain(vm, this)) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = BlockchainShowHistory {
        program: program(vm)?,
        blockchain,
        coin: name.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_blockchain_show_history(&message),
        |vm, history| match history {
            PriceHistory::Null => Ok::<_, IntrinsicError>(Value::Null),
            PriceHistory::Failure(failure) => {
                failure_value("show_history", &failure)
            }
            PriceHistory::Prices(prices) => {
                let heap = vm.heap_mut();
                let entries: Vec<(Value, Value)> = prices
                    .into_iter()
                    .enumerate()
                    .map(|(index, record)| {
                        let pair = heap.add_list(vec![record.price.into(), record.date.into()]);

                        (Value::from(index as f64), Value::List(pair))
                    })
                    .collect();

                Ok(Value::Map(heap.add_map_from(entries)))
            }
        },
    ))
}
