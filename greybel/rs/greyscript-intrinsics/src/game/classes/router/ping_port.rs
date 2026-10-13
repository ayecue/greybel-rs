use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::router_id;
use crate::error::IntrinsicError;
use crate::game::classes::port;
use crate::game::client::server::{program, respond, server};
use crate::protocol::RouterPingPort;

/// Returns router port `port` when the router forwards it.
#[native(method)]
pub fn ping_port(vm: &mut Vm, this: &Value, port: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(number) = port.as_int() else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let Some(router) = router_id(vm, this) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = RouterPingPort {
        program: program(vm)?,
        router,
        port: number,
    };

    Ok(respond(
        vm,
        server(vm)?.send_router_ping_port(&message),
        |vm, port| port.map_or(Value::Null, |port| port::create(vm, port)),
    ))
}
