use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::router_id;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::RouterDevicesLanIp;

/// Returns LAN addresses of devices behind the router.
#[native(method)]
pub fn devices_lan_ip(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(router) = router_id(vm, this) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = RouterDevicesLanIp {
        program: program(vm)?,
        router,
    };

    Ok(respond(
        vm,
        server(vm)?.send_router_devices_lan_ip(&message),
        |_, addresses| addresses,
    ))
}
