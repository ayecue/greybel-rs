use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::file;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::FileHasPermission;

#[native(method)]
pub fn has_permission(
    vm: &mut Vm,
    this: &Value,
    #[default("")]
    #[name = "typePerm"]
    permission: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let permission = vm.heap().display(permission).to_string();
    let Some(file) = file(vm, this, "has_permission") else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = FileHasPermission {
        program: program(vm)?,
        file: file.reference(),
        permission: permission.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_file_has_permission(&message),
        |_, permission| permission,
    ))
}
