use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};
use greyhack_behaviour::{MAX_NETWORK_COMPUTERS, is_lan_ip, is_valid_ip};

use super::is_admin;
use crate::error::IntrinsicError;
use crate::game::classes::router;
use crate::game::client::failure::failure_value;
use crate::game::client::options::{option, options, string_option};
use crate::game::client::server::{program, respond, server};
use crate::protocol::{AdminCreateNetwork, NetworkCreation, NetworkOptions};

/// Adds a network and returns its router object.
#[native(method)]
pub fn create_network(
    vm: &mut Vm,
    this: &Value,
    #[name = "options"] options_value: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    const NAME: &str = "create_network";

    if !is_admin(vm, this, NAME) {
        return Ok(NativeOutput::ready(Value::Null));
    }

    let map = options(options_value, NAME)?;
    let heap = vm.heap();
    let mut network = NetworkOptions {
        public_ip: string_option(heap, map, "public_ip", NAME)?,
        essid: string_option(heap, map, "essid", NAME)?,
        wifi_password: string_option(heap, map, "wifi_password", NAME)?,
        domain: string_option(heap, map, "domain", NAME)?,
        computers: None,
    };

    if let Some(count) = option(heap, map, "computers") {
        let count = count
            .as_int()
            .ok_or(IntrinsicError::InvalidArguments(NAME))?;

        network.computers = Some(u32::try_from(count).unwrap_or(0).min(MAX_NETWORK_COMPUTERS));
    }

    if let Some(address) = &network.public_ip
        && (!is_valid_ip(address) || is_lan_ip(address))
    {
        return Ok(NativeOutput::ready(format!(
            "Error: {address} is not a public address"
        )));
    }

    Ok(respond(
        vm,
        server(vm)?.send_admin_create_network(&AdminCreateNetwork {
            program: program(vm)?,
            network,
        }),
        |vm, creation| match creation {
            NetworkCreation::Router(id) => router::create(vm, id),
            NetworkCreation::Failure(failure) => {
                failure_value("create_network", &failure)
            }
        },
    ))
}
