use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::{device, use_camera};
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::TrafficNetCameraLinkSystem;

#[native(method)]
pub fn camera_link_system(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(traffic) = device(vm, this, "camera_link_system") else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = TrafficNetCameraLinkSystem {
        program: program(vm)?,
        traffic,
    };

    Ok(respond(
        vm,
        server(vm)?.send_traffic_net_camera_link_system(&message),
        |vm, outcome| use_camera(vm, "camera_link_system", outcome),
    ))
}
