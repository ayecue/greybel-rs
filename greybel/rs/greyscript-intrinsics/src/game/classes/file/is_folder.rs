use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::file;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::FileIsFolder;

#[native(method)]
pub fn is_folder(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(file) = file(vm, this, "is_folder") else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = FileIsFolder {
        program: program(vm)?,
        file: file.reference(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_file_is_folder(&message),
        |_, folder| folder,
    ))
}
