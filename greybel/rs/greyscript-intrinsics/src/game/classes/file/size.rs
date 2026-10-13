use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::file;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::FileSize;

/// Returns the byte size as a string.
#[native(method)]
pub fn size(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(file) = file(vm, this, "size") else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = FileSize {
        program: program(vm)?,
        file: file.reference(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_file_size(&message),
        |_, size| size,
    ))
}
