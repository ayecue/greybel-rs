use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::computer;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::ComputerDeleteUser;

#[native(method)]
pub fn delete_user(
    vm: &mut Vm,
    this: &Value,
    user: &Value,
    #[default(0)]
    #[name = "removeHome"]
    remove_home: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let (Some(name), Some(remove_home)) = (user.as_str(), remove_home.as_number()) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let remove_home = remove_home != 0.0;
    let Some(access) = computer(vm, this, "delete_user") else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    if name.is_empty() {
        return Err(IntrinsicError::InvalidArguments("delete_user"));
    }

    let message = ComputerDeleteUser {
        program: program(vm)?,
        computer: access.reference(),
        user: name.into(),
        remove_home,
    };

    Ok(respond(
        vm,
        server(vm)?.send_computer_delete_user(&message),
        |_, outcome| outcome.response("delete_user"),
    ))
}
