use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::is_admin;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::AdminRegisterPassword;

/// Registers `password` without an account and returns its MD5 hash (`Database.AddPassword`).
#[native(method)]
pub fn register_password(
    vm: &mut Vm,
    this: &Value,
    password: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    const NAME: &str = "register_password";

    if !is_admin(vm, this, NAME) {
        return Ok(NativeOutput::ready(Value::Null));
    }

    let Some(password) = password.as_str() else {
        return Err(IntrinsicError::InvalidArguments(NAME));
    };
    let message = AdminRegisterPassword {
        program: program(vm)?,
        password: password.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_register_password(&message),
        |_, hash: String| hash,
    ))
}
