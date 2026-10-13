//! Objects for police traffic networks.

use greybel_interpreter::{Native, Value, Vm};

use crate::error::IntrinsicError;
use crate::game::client::receiver::{self, receiver_for};
use crate::game::client::stopwatch;
use crate::protocol::{ObjectId, Outcome};
use crate::runtime::Handle;

mod camera_link_system;
mod get_credentials_info;
mod locate_vehicle;

pub(crate) const METHODS: &[(&str, &Native)] = &[
    (
        "camera_link_system",
        &camera_link_system::camera_link_system::NATIVE,
    ),
    ("locate_vehicle", &locate_vehicle::locate_vehicle::NATIVE),
    (
        "get_credentials_info",
        &get_credentials_info::get_credentials_info::NATIVE,
    ),
];

/// Stopwatch tracking the program's previous camera-system use (`TrafficNetIntrinsics.cronoLocateVehicle`).
struct CameraWatch;

/// Returns the server identity of the traffic-network device.
fn device(vm: &Vm, this: &Value, method: &str) -> Option<ObjectId> {
    receiver_for::<receiver::TrafficNet>(vm, this, method).map(Handle::reference)
}

/// Tests whether the program used the camera system within the last second and must still wait.
/// The game allows a use once more than a second has passed (`TrafficNetIntrinsics.camera_link_system`, `locate_vehicle`), so exactly a second still waits.
fn is_throttled(vm: &mut Vm) -> bool {
    stopwatch::elapsed_ms::<CameraWatch>(vm).is_some_and(|ms| ms <= 1000)
}

/// Restarts the camera stopwatch, or returns the too-many-calls text while the program must still wait.
fn throttle_camera(vm: &mut Vm) -> Result<(), Value> {
    if is_throttled(vm) {
        return Err("Error: Unable to use the camera at this time. Too many calls.".into());
    }

    stopwatch::restart::<CameraWatch>(vm);

    Ok(())
}

/// Returns the camera-system response for `intrinsic`, subject to world permission and the program's camera-use throttle.
fn use_camera(
    vm: &mut Vm,
    intrinsic: &'static str,
    outcome: Outcome,
) -> Result<Value, IntrinsicError> {
    match outcome {
        Outcome::True => Ok(throttle_camera(vm).map_or_else(|error| error, |()| Value::TRUE)),
        outcome => outcome.response(intrinsic),
    }
}
