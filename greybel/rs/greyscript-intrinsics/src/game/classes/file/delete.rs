use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::file;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::FileDelete;

/// Deletes the file; returns `""` on success or the failure reason.
#[native(method)]
pub fn delete(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(file) = file(vm, this, "delete") else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = FileDelete {
        program: program(vm)?,
        file: file.reference(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_file_delete(&message),
        |_, change| change.response("delete"),
    ))
}
