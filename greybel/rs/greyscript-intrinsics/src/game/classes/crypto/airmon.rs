use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::access_or_crash;
use crate::error::IntrinsicError;
use crate::game::client::object;
use crate::game::client::server::{program, respond, server};
use crate::protocol::CryptoAirmon;

/// Enables or disables monitor mode on a Wi-Fi card (`airmon`).
#[native(method)]
pub fn airmon(
    vm: &mut Vm,
    this: &Value,
    option: &Value,
    device: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    if !object::game_map(vm, this) {
        return Ok(NativeOutput::ready(false));
    }

    let (Some(option), Some(device)) = (option.as_str(), device.as_str()) else {
        return Ok(NativeOutput::ready(false));
    };

    if option.is_empty() || device.is_empty() || device.len() > 5 {
        return Ok(NativeOutput::ready(false));
    }

    let Some(access) = access_or_crash(vm, this)? else {
        return Ok(NativeOutput::ready(false));
    };

    let message = CryptoAirmon {
        program: program(vm)?,
        crypto: access.reference(),
        start: option == "start",
        device: device.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_crypto_airmon(&message),
        |_, outcome| outcome.response("airmon"),
    ))
}
