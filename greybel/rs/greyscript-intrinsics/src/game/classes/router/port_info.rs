use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use crate::error::IntrinsicError;
use crate::game::client::receiver;
use crate::game::client::receiver::{receiver_for, receiver_or_crash};
use crate::game::client::server::{program, respond, server};
use crate::protocol::RouterPortInfo;

/// Returns the port's current service and library version as resolved by the router.
/// Returns `unknown unknown` when neither resolves (`Router.GetPortVersion`).
#[native(method)]
pub fn port_info(vm: &mut Vm, this: &Value, port: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(port) = receiver_for::<receiver::Port>(vm, port, "port_number") else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let Some(router) = receiver_or_crash::<receiver::Router>(vm, this)? else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let router = router.reference();

    if port.lan_ip.is_empty() {
        return Ok(NativeOutput::ready(Value::from("unknown unknown")));
    }

    let message = RouterPortInfo {
        program: program(vm)?,
        router,
        lan_ip: port.lan_ip.clone().into(),
        internal_port: port.internal_port,
    };

    Ok(respond(
        vm,
        server(vm)?.send_router_port_info(&message),
        |_, service| {
            service.map_or_else(
                || "unknown unknown".to_owned(),
                |service| format!("{} {}", service.service, service.version),
            )
        },
    ))
}
