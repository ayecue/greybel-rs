use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::sub_wallet;
use crate::error::IntrinsicError;
use crate::game::client::failure::failure_value;
use crate::game::client::server::{program, respond, server};
use crate::protocol::{Figure, SubWalletGetBalance};

#[native(method)]
pub fn get_balance_subwallet(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(sub_wallet) = sub_wallet(vm, this) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = SubWalletGetBalance {
        program: program(vm)?,
        sub_wallet,
    };

    Ok(respond(
        vm,
        server(vm)?.send_sub_wallet_get_balance(&message),
        |_, balance| match balance {
            Figure::Null => Ok::<_, IntrinsicError>(Value::Null),
            Figure::Number(balance) => Ok(Value::from(balance)),
            Figure::Failure(failure) => failure_value("get_balance", &failure),
        },
    ))
}
