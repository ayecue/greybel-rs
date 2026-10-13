use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};
use greyhack_behaviour::check_new_user;

use super::computer_with_strings;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, refuse, respond, server};
use crate::protocol::ComputerCreateUser;

#[native(method)]
pub fn create_user(
    vm: &mut Vm,
    this: &Value,
    user: &Value,
    password: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let Some((access, user, password)) = computer_with_strings(vm, this, "create_user", user, password) else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    if let Err(refusal) = check_new_user(user, password) {
        return refuse("create_user", refusal);
    }

    let message = ComputerCreateUser {
        program: program(vm)?,
        computer: access.reference(),
        user: user.into(),
        password: password.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_computer_create_user(&message),
        |_, outcome| outcome.response("create_user"),
    ))
}
