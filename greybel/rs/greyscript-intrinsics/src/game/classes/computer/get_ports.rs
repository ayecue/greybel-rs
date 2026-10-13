use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::computer;
use crate::error::IntrinsicError;
use crate::game::classes::port;
use crate::game::client::server::{program, respond, server};
use crate::protocol::ComputerGetPorts;

#[native(method)]
pub fn get_ports(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(access) = computer(vm, this, "get_ports") else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = ComputerGetPorts {
        program: program(vm)?,
        computer: access.reference(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_computer_get_ports(&message),
        |vm, ports| {
            ports.map(|ports| {
                ports
                    .into_iter()
                    .map(|port| port::create(vm, port))
                    .collect::<Vec<_>>()
            })
        },
    ))
}
