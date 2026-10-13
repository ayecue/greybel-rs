use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::file;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::FileName;

#[native(method)]
pub fn name(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(file) = file(vm, this, "name") else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = FileName {
        program: program(vm)?,
        file: file.reference(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_file_name(&message),
        |_, name| name,
    ))
}
