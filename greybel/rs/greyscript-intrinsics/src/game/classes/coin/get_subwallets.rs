use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::coin_id;
use crate::error::IntrinsicError;
use crate::game::classes::sub_wallet;
use crate::game::client::server::{program, respond, server};
use crate::protocol::{CoinGetSubwallets, ObjectId};

#[native(method)]
pub fn get_subwallets(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(id) = coin_id(vm, this) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = CoinGetSubwallets {
        program: program(vm)?,
        coin: id,
    };

    Ok(respond(
        vm,
        server(vm)?.send_coin_get_subwallets(&message),
        |vm, sub_wallets: Vec<ObjectId>| {
            let mut objects = Vec::with_capacity(sub_wallets.len());

            for sub_wallet in sub_wallets {
                objects.push(sub_wallet::create(vm, sub_wallet)?);
            }

            Ok::<_, IntrinsicError>(Value::List(vm.heap_mut().add_list(objects)))
        },
    ))
}
