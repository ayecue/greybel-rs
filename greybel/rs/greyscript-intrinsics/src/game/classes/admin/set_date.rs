use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::is_admin;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::AdminSetDate;

/// Sets the world date returned by `current_date`.
#[native(method)]
pub fn set_date(vm: &mut Vm, this: &Value, date: &Value) -> Result<NativeFuture, IntrinsicError> {
    if !is_admin(vm, this, "set_date") {
        return Ok(NativeOutput::ready(Value::Null));
    }

    let Some(date) = date.as_str() else {
        return Err(IntrinsicError::InvalidArguments("set_date"));
    };
    let message = AdminSetDate {
        program: program(vm)?,
        date: date.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_set_date(&message),
        |_, set: bool| set,
    ))
}
