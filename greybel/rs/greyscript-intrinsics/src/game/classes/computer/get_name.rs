use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::computer;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::ComputerGetName;

#[native(method)]
pub fn get_name(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(access) = computer(vm, this, "get_name") else {
        return Ok(NativeOutput::ready("Unknown"));
    };
    let message = ComputerGetName {
        program: program(vm)?,
        computer: access.reference(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_computer_get_name(&message),
        |_, name: Option<String>| name.unwrap_or_else(|| "Unknown".to_owned()),
    ))
}
