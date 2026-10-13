use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::is_admin;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::AdminCreateProgram;

/// Sandbox-only helper for writing executable source files.
#[native(method)]
pub fn create_program(
    vm: &mut Vm,
    this: &Value,
    path: &Value,
    source: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    if !is_admin(vm, this, "create_program") {
        return Ok(NativeOutput::ready(Value::Null));
    }

    let (Some(path), Some(source)) = (path.as_str(), source.as_str()) else {
        return Err(IntrinsicError::InvalidArguments("create_program"));
    };
    let message = AdminCreateProgram {
        program: program(vm)?,
        path: path.into(),
        source: source.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_create_program(&message),
        |_, created: bool| created,
    ))
}
