use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::is_admin;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::AdminCreateCtfAccount;

#[native(method)]
pub fn create_ctf_account(
    vm: &mut Vm,
    this: &Value,
    user: &Value,
    password: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    if !is_admin(vm, this, "create_ctf_account") {
        return Ok(NativeOutput::ready(Value::Null));
    }

    let (Some(user), Some(password)) = (user.as_str(), password.as_str()) else {
        return Err(IntrinsicError::InvalidArguments("create_ctf_account"));
    };
    let message = AdminCreateCtfAccount {
        program: program(vm)?,
        user: user.into(),
        password: password.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_create_ctf_account(&message),
        |_, created: bool| created,
    ))
}
