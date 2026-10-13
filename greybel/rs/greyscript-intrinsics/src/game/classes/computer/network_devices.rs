use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::computer;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::{ComputerNetworkDevices, NetworkDevice};

/// Returns `ethN`/`wlanN`, the card description's first word, and monitor mode formatted as a .NET boolean.
#[native(method)]
pub fn network_devices(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(access) = computer(vm, this, "network_devices") else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = ComputerNetworkDevices {
        program: program(vm)?,
        computer: access.reference(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_computer_network_devices(&message),
        |_, devices: Option<Vec<NetworkDevice>>| {
            devices.map(|devices| {
                devices
                    .iter()
                    .map(|device| {
                        let monitor = if device.monitor { "True" } else { "False" };

                        format!("{} {} {monitor}\n", device.name, device.model)
                    })
                    .collect::<String>()
            })
        },
    ))
}
