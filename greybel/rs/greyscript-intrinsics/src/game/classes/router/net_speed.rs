use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::router_id;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::RouterNetSpeed;

/// Returns the router network's connection speed (`RouterIntrinsics.net_speed`).
#[native(method)]
pub fn net_speed(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(router) = router_id(vm, this) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = RouterNetSpeed {
        program: program(vm)?,
        router,
    };

    Ok(respond(
        vm,
        server(vm)?.send_router_net_speed(&message),
        |_, speed: Option<u8>| speed.map(f64::from),
    ))
}
