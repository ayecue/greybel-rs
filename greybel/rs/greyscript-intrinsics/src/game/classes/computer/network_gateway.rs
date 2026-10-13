use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::computer;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::ComputerNetworkGateway;

#[native(method)]
pub fn network_gateway(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(access) = computer(vm, this, "network_gateway") else {
        return Ok(NativeOutput::ready(greyhack_behaviour::NO_ADDRESS));
    };
    let message = ComputerNetworkGateway {
        program: program(vm)?,
        computer: access.reference(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_computer_network_gateway(&message),
        |_, gateway: Option<String>| {
            gateway.unwrap_or_else(|| greyhack_behaviour::NO_ADDRESS.to_owned())
        },
    ))
}
