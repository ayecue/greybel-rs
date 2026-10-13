use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::file;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::FilePermissions;

#[native(method)]
pub fn permissions(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(file) = file(vm, this, "permissions") else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = FilePermissions {
        program: program(vm)?,
        file: file.reference(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_file_permissions(&message),
        |_, permissions| permissions,
    ))
}
