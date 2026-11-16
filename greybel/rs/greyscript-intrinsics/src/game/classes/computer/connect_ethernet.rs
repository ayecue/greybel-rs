use greybel_interpreter::{NativeFuture, Value, Vm, native};

use super::{computer, restart_switch_watch, switch_wait};
use crate::error::IntrinsicError;
use crate::game::client::server::{program, raise, server, settle};
use crate::protocol::{ComputerConnectEthernet, EthernetSettings};

#[native(method)]
pub fn connect_ethernet(
    vm: &mut Vm,
    this: &Value,
    net_device: &Value,
    address: &Value,
    gateway: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    if !matches!(this, Value::Map(_))
        || net_device.as_str().is_none()
        || address.as_str().is_none()
        || gateway.as_str().is_none()
    {
        return Err(IntrinsicError::InvalidArguments("connect_ethernet"));
    }

    let Some(access) = computer(vm, this, "connect_ethernet") else {
        return Err(IntrinsicError::InvalidArguments("connect_ethernet"));
    };
    let settings = Some(EthernetSettings {
        device: net_device.as_str().unwrap().to_owned(),
        address: address.as_str().unwrap().to_owned(),
        gateway: gateway.as_str().unwrap().to_owned(),
    });
    let (now, wait) = switch_wait(vm);
    let message = ComputerConnectEthernet {
        program: program(vm)?,
        computer: access.reference(),
        settings,
        wait,
    };

    Ok(settle(
        server(vm)?.send_computer_connect_ethernet(&message),
        move |vm, connection| {
            let connection = connection.map_err(|failure| raise("connect_ethernet", failure))?;

            restart_switch_watch(vm, now, connection.starts_wait);

            connection.outcome.into_value_or_empty("connect_ethernet")
        },
    ))
}
