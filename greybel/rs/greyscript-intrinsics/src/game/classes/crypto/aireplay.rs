use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::access_or_crash;
use crate::error::IntrinsicError;
use crate::game::client::failure::failure_value;
use crate::game::client::object;
use crate::game::client::server::{program, respond, server};
use crate::game::{game, send_output};
use crate::protocol::{Capture, CryptoAireplay};

/// Captures Wi-Fi ACKs into `file.cap` for `aircrack` (`aireplay`).
#[native(method)]
pub fn aireplay(
    vm: &mut Vm,
    this: &Value,
    bssid: &Value,
    essid: &Value,
    #[default(-1)]
    #[name = "maxAcks"]
    max_acks: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    if !object::game_map(vm, this) {
        return Ok(NativeOutput::ready(Value::Null));
    }

    let (Some(bssid), Some(essid), Some(max_acks)) =
        (bssid.as_str(), essid.as_str(), max_acks.as_int())
    else {
        return Err(IntrinsicError::InvalidArguments("aireplay"));
    };

    if bssid.is_empty() || essid.is_empty() {
        return Err(IntrinsicError::InvalidArguments("aireplay"));
    }

    let Some(access) = access_or_crash(vm, this)? else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    let message = CryptoAireplay {
        program: program(vm)?,
        crypto: access.reference(),
        bssid: bssid.to_uppercase().into(),
        essid: essid.into(),
        max_acks,
        folder: game(vm)?.session.current_path.clone().into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_crypto_aireplay(&message),
        |vm, capture| {
            Ok::<_, IntrinsicError>(match capture {
                // Grey Hack flushes output when capture starts; capture returns null.
                Capture::Null => {
                    send_output(vm);

                    Value::Null
                }
                Capture::Failure(failure) => failure_value("aireplay", &failure)?,
            })
        },
    ))
}
