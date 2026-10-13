use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::is_admin;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::AdminNetworks;

/// Returns every network's public address.
#[native(method)]
pub fn networks(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    if !is_admin(vm, this, "networks") {
        return Ok(NativeOutput::ready(Value::Null));
    }

    Ok(respond(
        vm,
        server(vm)?.send_admin_networks(&AdminNetworks {
            program: program(vm)?,
        }),
        |_, addresses: Vec<String>| addresses,
    ))
}
