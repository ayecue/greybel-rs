use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::computer;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::ComputerIsNetworkActive;

#[native(method)]
pub fn is_network_active(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(access) = computer(vm, this, "is_network_active") else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = ComputerIsNetworkActive {
        program: program(vm)?,
        computer: access.reference(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_computer_is_network_active(&message),
        |_, active| active,
    ))
}
