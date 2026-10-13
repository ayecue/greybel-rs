use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::file;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::FileOwner;

#[native(method)]
pub fn owner(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(file) = file(vm, this, "owner") else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = FileOwner {
        program: program(vm)?,
        file: file.reference(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_file_owner(&message),
        |_, owner| owner,
    ))
}
