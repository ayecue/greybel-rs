use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::file;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::FileIsBinary;

#[native(method)]
pub fn is_binary(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(file) = file(vm, this, "is_binary") else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = FileIsBinary {
        program: program(vm)?,
        file: file.reference(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_file_is_binary(&message),
        |_, binary| binary,
    ))
}
