use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::public_ip;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::AdminRemoveNetwork;

/// Removes this non-player network and all its computers.
#[native(method)]
pub fn remove(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(public_ip) = public_ip(vm, this, "remove") else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = AdminRemoveNetwork {
        program: program(vm)?,
        public_ip: public_ip.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_remove_network(&message),
        |_, removed: bool| removed,
    ))
}
