use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::access;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, raise, server, settle};
use crate::protocol::AptClientUpdate;

/// Prints one update line per source and returns an empty string.
#[native(method)]
pub fn update(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(access) = access(vm, this) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = AptClientUpdate {
        program: program(vm)?,
        apt_client: access.reference(),
    };

    Ok(settle(
        server(vm)?.send_apt_client_update(&message),
        |_vm, update| {
            update.map_err(|failure| raise("update", failure))?;

            Ok::<_, IntrinsicError>(String::new())
        },
    ))
}
