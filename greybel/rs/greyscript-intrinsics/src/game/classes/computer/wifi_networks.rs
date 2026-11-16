use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::computer;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, raise, server, settle};
use crate::protocol::{ComputerWifiNetworks, WifiScan};

#[native(method)]
pub fn wifi_networks(
    vm: &mut Vm,
    this: &Value,
    net_device: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let Some(access) = computer(vm, this, "wifi_networks") else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let Some(device) = net_device.as_str() else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    if device.len() < 4 {
        return Ok(NativeOutput::ready(Value::Null));
    }

    let name = device.to_owned();
    let message = ComputerWifiNetworks {
        program: program(vm)?,
        computer: access.reference(),
        device: device.into(),
    };

    Ok(settle(
        server(vm)?.send_computer_wifi_networks(&message),
        move |vm, networks| {
            networks
                .map(|scan| scan.map(|scan| network_value(vm, &name, scan)))
                .map_err(|failure| raise("wifi_networks", failure))
        },
    ))
}

/// Lists networks as `<bssid> <power>% <essid>`, or returns the game's string for a missing or non-Wi-Fi card
/// `device`.
fn network_value(vm: &mut Vm, device: &str, scan: WifiScan) -> Value {
    match scan {
        WifiScan::NoDevice => Value::from(format!("wifi_networks: {device} not found.")),
        WifiScan::NotWifi => Value::from(format!("wifi_networks: {device} is not a wifi card.")),
        WifiScan::Networks(networks) => {
            let networks = networks
                .into_iter()
                .map(|network| {
                    Value::from(format!(
                        "{} {}% {}",
                        network.bssid, network.power, network.essid
                    ))
                })
                .collect();

            Value::List(vm.heap_mut().add_list(networks))
        }
    }
}
