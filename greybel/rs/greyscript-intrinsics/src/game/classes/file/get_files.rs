use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::{file, file_objects};
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::FileGetFiles;

#[native(method)]
pub fn get_files(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(file) = file(vm, this, "get_files") else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = FileGetFiles {
        program: program(vm)?,
        file: file.reference(),
    };
    let file = file.clone();

    Ok(respond(
        vm,
        server(vm)?.send_file_get_files(&message),
        move |vm, files| file_objects(vm, &file, files),
    ))
}
