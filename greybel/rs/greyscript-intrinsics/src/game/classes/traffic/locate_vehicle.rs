use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::{device, is_throttled, use_camera};
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::TrafficNetLocateVehicle;

#[native(method)]
pub fn locate_vehicle(
    vm: &mut Vm,
    this: &Value,
    #[name = "license_plate"] license_plate: &Value,
    password: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let (Some(plate), Some(password)) = (license_plate.as_str(), password.as_str()) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let Some(traffic) = device(vm, this, "locate_vehicle") else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = TrafficNetLocateVehicle {
        program: program(vm)?,
        traffic,
        license_plate: plate.into(),
        password: password.into(),
        throttled: is_throttled(vm),
    };

    Ok(respond(
        vm,
        server(vm)?.send_traffic_net_locate_vehicle(&message),
        |vm, outcome| use_camera(vm, "locate_vehicle", outcome),
    ))
}
