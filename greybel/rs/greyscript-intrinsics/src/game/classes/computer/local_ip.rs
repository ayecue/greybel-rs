use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::computer;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::ComputerLocalIp;

#[native(method)]
pub fn local_ip(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(access) = computer(vm, this, "local_ip") else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = ComputerLocalIp {
        program: program(vm)?,
        computer: access.reference(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_computer_local_ip(&message),
        |_, address| address,
    ))
}
