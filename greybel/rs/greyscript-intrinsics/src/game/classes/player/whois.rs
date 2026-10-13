use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};
use greyhack_behaviour::{is_lan_ip, is_valid_ip};

use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::PlayerWhois;

/// Returns the owner of the network at public address `address`.
#[native]
pub fn whois(vm: &mut Vm, address: &Value) -> Result<NativeFuture, IntrinsicError> {
    let address = match address.as_str() {
        Some(address) if !address.is_empty() => address,
        _ => return Err(IntrinsicError::InvalidArguments("whois")),
    };

    if !is_valid_ip(address) {
        return Ok(NativeOutput::ready(format!(
            "Invalid IP address: {address}"
        )));
    }

    if is_lan_ip(address) {
        return Ok(NativeOutput::ready("Error: the IP address must be public"));
    }

    let message = PlayerWhois {
        program: program(vm)?,
        address: address.to_owned().into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_player_whois(&message),
        |_, owner| owner.response(),
    ))
}
