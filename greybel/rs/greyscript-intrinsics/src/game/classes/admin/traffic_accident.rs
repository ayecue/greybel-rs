use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::is_admin;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::AdminTrafficAccident;

/// Reproduces the client-triggered accident for the car last found by `locate_vehicle` (`PlayerHelperServer.EnableCarAccidentServerRpc`).
/// If its driver is commuting and the police network's traffic override is enabled and unused, the driver dies, the override is consumed, and cameras cool down for one day.
/// Returns whether the driver died.
#[native(method)]
pub fn traffic_accident(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    if !is_admin(vm, this, "traffic_accident") {
        return Ok(NativeOutput::ready(Value::Null));
    }

    Ok(respond(
        vm,
        server(vm)?.send_admin_traffic_accident(&AdminTrafficAccident {
            program: program(vm)?,
        }),
        |_, applied: bool| applied,
    ))
}
