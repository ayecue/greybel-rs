use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};
use greyhack_behaviour::check_password;

use super::computer_with_strings;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, refuse, respond, server};
use crate::protocol::ComputerChangePassword;

#[native(method)]
pub fn change_password(
    vm: &mut Vm,
    this: &Value,
    user: &Value,
    password: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let Some((access, user, password)) =
        computer_with_strings(vm, this, "change_password", user, password)
    else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    if user.is_empty() {
        return Err(IntrinsicError::InvalidArguments("change_password"));
    }

    if let Err(refusal) = check_password(password) {
        return refuse("change_password", refusal);
    }

    let message = ComputerChangePassword {
        program: program(vm)?,
        computer: access.reference(),
        user: user.into(),
        password: password.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_computer_change_password(&message),
        |_, outcome| outcome.response("change_password"),
    ))
}
