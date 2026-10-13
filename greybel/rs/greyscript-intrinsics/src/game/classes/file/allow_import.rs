use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::file;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::FileAllowImport;

#[native(method)]
pub fn allow_import(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(file) = file(vm, this, "allow_import") else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = FileAllowImport {
        program: program(vm)?,
        file: file.reference(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_file_allow_import(&message),
        |_, permission| permission,
    ))
}
