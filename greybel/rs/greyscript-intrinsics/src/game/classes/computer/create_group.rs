use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};
use greyhack_behaviour::check_new_group;

use super::computer_with_strings;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, refuse, respond, server};
use crate::protocol::ComputerCreateGroup;

#[native(method)]
pub fn create_group(
    vm: &mut Vm,
    this: &Value,
    username: &Value,
    groupname: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let Some((access, user, group)) = computer_with_strings(vm, this, "create_group", username, groupname) else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    if let Err(refusal) = check_new_group(user, group) {
        return refuse("create_group", refusal);
    }

    let message = ComputerCreateGroup {
        program: program(vm)?,
        computer: access.reference(),
        user: user.into(),
        group: group.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_computer_create_group(&message),
        |_, outcome| outcome.response("create_group"),
    ))
}
