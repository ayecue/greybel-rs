use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::access_or_crash;
use crate::error::IntrinsicError;
use crate::game::client::object;
use crate::game::client::server::{program, respond, server};
use crate::protocol::{CryptoAircrack, KeyRecovery};

/// Recovers a Wi-Fi password from `file.cap` (`aircrack`).
/// Returns the password when the capture has enough ACKs; otherwise prints Grey Hack's message and returns `null`.
#[native(method)]
pub fn aircrack(
    vm: &mut Vm,
    this: &Value,
    #[name = "pathFile"] path_file: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    if !object::game_map(vm, this) {
        return Ok(NativeOutput::ready(Value::Null));
    }

    let Some(path) = path_file.as_str() else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    if path.is_empty() {
        return Err(IntrinsicError::InvalidArguments("aircrack"));
    }

    let Some(access) = access_or_crash(vm, this)? else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    let message = CryptoAircrack {
        program: program(vm)?,
        crypto: access.reference(),
        path: path.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_crypto_aircrack(&message),
        |_vm, recovery| {
            Ok::<_, IntrinsicError>(match recovery {
                KeyRecovery::Null => Value::Null,
                KeyRecovery::Password(password) => Value::from(password),
            })
        },
    ))
}
