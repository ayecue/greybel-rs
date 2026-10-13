use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::{file, file_objects};
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::FileGetFolders;

#[native(method)]
pub fn get_folders(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(file) = file(vm, this, "get_folders") else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = FileGetFolders {
        program: program(vm)?,
        file: file.reference(),
    };
    let file = file.clone();

    Ok(respond(
        vm,
        server(vm)?.send_file_get_folders(&message),
        move |vm, folders| file_objects(vm, &file, folders),
    ))
}
