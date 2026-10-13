use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::access;
use crate::error::IntrinsicError;
use crate::game::client::login_encryption::LoginEncryption;
use crate::game::client::object;
use crate::game::client::server::{program, respond, server};
use crate::game::send_output;
use crate::protocol::CryptoDecipher;

/// Recovers a password through reverse lookup across world users.
#[native(method)]
pub fn decipher(
    vm: &mut Vm,
    this: &Value,
    #[name = "encryptPass"] hash: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    if !object::game_map(vm, this) || hash.as_str().is_none() {
        return Ok(NativeOutput::ready(Value::Null));
    }

    LoginEncryption::deny_in(vm, "decipher")?;

    if access(vm, this).is_none() {
        return Ok(NativeOutput::ready(Value::Null));
    }

    let hash = hash.as_str().expect("the hash was checked above");

    // Grey Hack flushes output whenever decipher starts (`GreyInterpreter.CheckDecipher`).
    send_output(vm);

    let message = CryptoDecipher {
        program: program(vm)?,
        hash: hash.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_crypto_decipher(&message),
        |_, password| password,
    ))
}
