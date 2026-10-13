use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::computer;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::ComputerPublicIp;

#[native(method)]
pub fn public_ip(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(access) = computer(vm, this, "public_ip") else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = ComputerPublicIp {
        program: program(vm)?,
        computer: access.reference(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_computer_public_ip(&message),
        |_, address| address,
    ))
}
