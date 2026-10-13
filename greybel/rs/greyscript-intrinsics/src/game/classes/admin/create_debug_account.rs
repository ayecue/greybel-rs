use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::is_admin;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::AdminCreateDebugAccount;

#[native(method)]
pub fn create_debug_account(
    vm: &mut Vm,
    this: &Value,
    user: &Value,
    password: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    const NAME: &str = "create_debug_account";

    if !is_admin(vm, this, NAME) {
        return Ok(NativeOutput::ready(Value::Null));
    }

    let (Some(user), Some(password)) = (user.as_str(), password.as_str()) else {
        return Err(IntrinsicError::InvalidArguments(NAME));
    };
    let message = AdminCreateDebugAccount {
        program: program(vm)?,
        user: user.into(),
        password: password.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_create_debug_account(&message),
        |_, created: bool| created,
    ))
}
