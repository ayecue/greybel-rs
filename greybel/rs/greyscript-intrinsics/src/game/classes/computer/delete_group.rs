use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::computer_with_strings;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::ComputerDeleteGroup;

#[native(method)]
pub fn delete_group(
    vm: &mut Vm,
    this: &Value,
    username: &Value,
    groupname: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let Some((access, user, group)) =
        computer_with_strings(vm, this, "delete_group", username, groupname)
    else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    if user.is_empty() || group.is_empty() {
        return Err(IntrinsicError::InvalidArguments("delete_group"));
    }

    let message = ComputerDeleteGroup {
        program: program(vm)?,
        computer: access.reference(),
        user: user.into(),
        group: group.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_computer_delete_group(&message),
        |_, outcome| outcome.response("delete_group"),
    ))
}
