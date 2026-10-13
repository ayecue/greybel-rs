use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::is_admin;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::AdminDeposit;

/// Adds `amount` to bank account `number`; not available in Grey Hack.
#[native(method)]
pub fn deposit(
    vm: &mut Vm,
    this: &Value,
    number: &Value,
    amount: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    if !is_admin(vm, this, "deposit") {
        return Ok(NativeOutput::ready(Value::Null));
    }

    let (Some(number), Some(amount)) = (number.as_str(), amount.as_number()) else {
        return Err(IntrinsicError::InvalidArguments("deposit"));
    };
    let message = AdminDeposit {
        program: program(vm)?,
        number: number.into(),
        amount,
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_deposit(&message),
        |_, deposited: bool| deposited,
    ))
}
