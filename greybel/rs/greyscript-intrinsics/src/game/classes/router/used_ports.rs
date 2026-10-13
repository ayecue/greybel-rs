use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::router_id;
use crate::error::IntrinsicError;
use crate::game::classes::port;
use crate::game::client::server::{program, respond, server};
use crate::protocol::RouterUsedPorts;

/// Returns externally visible ports forwarded by the router.
#[native(method)]
pub fn used_ports(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(router) = router_id(vm, this) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = RouterUsedPorts {
        program: program(vm)?,
        router,
    };

    Ok(respond(
        vm,
        server(vm)?.send_router_used_ports(&message),
        |vm, ports| {
            ports.map(|ports| {
                ports
                    .into_iter()
                    .map(|port| port::create(vm, port))
                    .collect::<Vec<_>>()
            })
        },
    ))
}
