use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::{computer, restart_switch_watch, switch_wait};
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::{ComputerConnectEthernet, EthernetSettings};

#[native(method)]
pub fn connect_ethernet(
    vm: &mut Vm,
    this: &Value,
    net_device: &Value,
    address: &Value,
    gateway: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let Some(access) = computer(vm, this, "connect_ethernet") else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let settings = match (net_device.as_str(), address.as_str(), gateway.as_str()) {
        (Some(device), Some(address), Some(gateway)) => Some(EthernetSettings {
            device: device.to_owned(),
            address: address.to_owned(),
            gateway: gateway.to_owned(),
        }),
        _ => None,
    };
    let (now, wait) = switch_wait(vm);
    let message = ComputerConnectEthernet {
        program: program(vm)?,
        computer: access.reference(),
        settings,
        wait,
    };

    Ok(respond(
        vm,
        server(vm)?.send_computer_connect_ethernet(&message),
        move |vm, connection| {
            restart_switch_watch(vm, now, connection.starts_wait);

            connection.outcome.into_value_or_empty("connect_ethernet")
        },
    ))
}
