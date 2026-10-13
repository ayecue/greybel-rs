use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::sub_wallet;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::SubWalletLastTransaction;

/// Returns the last transfer as `[destAccount, amount, direction, date]`, or `false` if none exists.
#[native(method)]
pub fn last_transaction(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(sub_wallet) = sub_wallet(vm, this) else {
        return Ok(NativeOutput::ready(Value::FALSE));
    };
    let message = SubWalletLastTransaction {
        program: program(vm)?,
        sub_wallet,
    };

    Ok(respond(
        vm,
        server(vm)?.send_sub_wallet_last_transaction(&message),
        |vm, last| {
            let Some(last) = last else {
                return Value::FALSE;
            };
            let entry = vec![
                Value::from(last.dest_account),
                Value::from(last.amount),
                Value::from(f64::from(last.direction)),
                Value::from(last.date),
            ];

            Value::List(vm.heap_mut().add_list(entry))
        },
    ))
}
