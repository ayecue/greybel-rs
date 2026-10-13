use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::router_id;
use crate::error::IntrinsicError;
use crate::game::classes::port;
use crate::game::client::server::{program, respond, server};
use crate::protocol::{DevicePortList, RouterDevicePorts};

/// Returns ports for device `address` behind the router.
/// Queries from another network are restricted to devices known to the router.
#[native(method)]
pub fn device_ports(
    vm: &mut Vm,
    this: &Value,
    address: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let Some(address) = address.as_str() else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    if address.is_empty() {
        return Err(IntrinsicError::InvalidArguments("device_ports"));
    }

    let Some(router) = router_id(vm, this) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = RouterDevicePorts {
        program: program(vm)?,
        router,
        address: address.into(),
    };
    let address = address.to_owned();

    Ok(respond(
        vm,
        server(vm)?.send_router_device_ports(&message),
        move |vm, ports| match ports {
            DevicePortList::Null => Value::Null,
            DevicePortList::Unreachable => {
                Value::from(format!("device_ports: {address} is unreachable"))
            }
            DevicePortList::Ports(ports) => {
                let ports: Vec<Value> = ports
                    .into_iter()
                    .map(|port| port::create(vm, port))
                    .collect();

                Value::List(vm.heap_mut().add_list(ports))
            }
        },
    ))
}
