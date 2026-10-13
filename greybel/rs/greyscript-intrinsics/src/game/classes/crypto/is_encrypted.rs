use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::access_or_crash;
use crate::error::IntrinsicError;
use crate::game::client::object;
use crate::game::client::server::{program, respond, server};
use crate::protocol::CryptoIsEncrypted;

/// Tests whether the file at `filePath` is encrypted.
#[native(method)]
pub fn is_encrypted(
    vm: &mut Vm,
    this: &Value,
    #[name = "filePath"] path: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    if !object::game_map(vm, this) {
        return Ok(NativeOutput::ready(Value::Null));
    }

    let Some(path) = path.as_str() else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    let Some(access) = access_or_crash(vm, this)? else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    if path.is_empty() {
        return Ok(NativeOutput::ready("is_encrypted: Invalid file path"));
    }

    let message = CryptoIsEncrypted {
        program: program(vm)?,
        crypto: access.reference(),
        path: path.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_crypto_is_encrypted(&message),
        |_, outcome| outcome.response("is_encrypted"),
    ))
}
