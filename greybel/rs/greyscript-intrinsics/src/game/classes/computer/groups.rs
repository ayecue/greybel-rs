use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::computer;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::ComputerGroups;

#[native(method)]
pub fn groups(vm: &mut Vm, this: &Value, username: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(name) = username.as_str() else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let Some(access) = computer(vm, this, "groups") else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    if name.is_empty() {
        return Err(IntrinsicError::InvalidArguments("groups"));
    }

    let message = ComputerGroups {
        program: program(vm)?,
        computer: access.reference(),
        user: name.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_computer_groups(&message),
        |_, groups| groups.response("groups"),
    ))
}
