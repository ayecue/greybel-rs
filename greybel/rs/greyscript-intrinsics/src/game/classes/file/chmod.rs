use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::file;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::FileChmod;

/// Implements `chmod("u+rwx")`; returns `""` on success or the failure reason.
#[native(method)]
pub fn chmod(
    vm: &mut Vm,
    this: &Value,
    #[default("")]
    #[name = "typePerm"]
    permissions: &Value,
    #[default(0)]
    #[name = "isRecursive"]
    recursive: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let Some(file) = file(vm, this, "chmod") else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let change = vm.heap().display(permissions).to_string();
    let Some(recursive) = recursive.as_int() else {
        return Err(IntrinsicError::InvalidArguments("chmod"));
    };
    let message = FileChmod {
        program: program(vm)?,
        file: file.reference(),
        change: change.into(),
        recursive: recursive == 1,
    };

    Ok(respond(
        vm,
        server(vm)?.send_file_chmod(&message),
        |_, change| change.response("chmod"),
    ))
}
