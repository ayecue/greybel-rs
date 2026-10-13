use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::{computer, restart_switch_watch, switch_wait};
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::ComputerConnectWifi;

#[native(method)]
pub fn connect_wifi(
    vm: &mut Vm,
    this: &Value,
    net_device: &Value,
    bssid: &Value,
    essid: &Value,
    password: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let Some(access) = computer(vm, this, "connect_wifi") else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    let (now, wait) = switch_wait(vm);
    let message = ComputerConnectWifi {
        program: program(vm)?,
        computer: access.reference(),
        device: net_device.as_str().map(Into::into),
        bssid: bssid.as_str().map(Into::into),
        essid: essid.as_str().map(Into::into),
        password: password.as_str().map(Into::into),
        wait,
    };

    Ok(respond(
        vm,
        server(vm)?.send_computer_connect_wifi(&message),
        move |vm, connection| {
            restart_switch_watch(vm, now, connection.starts_wait);

            connection.outcome.response("connect_wifi")
        },
    ))
}
