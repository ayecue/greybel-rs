use greybel_interpreter::{NativeFuture, Value, Vm, native};

use super::{FileKey, file_key};
use crate::error::IntrinsicError;
use crate::game::client::check::proceed;
use crate::game::client::server::{program, respond, server};
use crate::protocol::CryptoDecrypt;

/// Decrypts `filePath` when `password` matches.
/// Returns `true` on success or Grey Hack's failure message.
#[native(method)]
pub fn decrypt(
    vm: &mut Vm,
    this: &Value,
    #[name = "filePath"] path: &Value,
    password: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let FileKey {
        access,
        path,
        password,
    } = proceed!(file_key(vm, this, "decrypt", path, password));

    let message = CryptoDecrypt {
        program: program(vm)?,
        crypto: access.reference(),
        path: path.into(),
        password: password.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_crypto_decrypt(&message),
        |_, outcome| outcome.response("decrypt"),
    ))
}
