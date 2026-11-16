use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::access;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, raise, server, settle};
use crate::protocol::AptClientUpdate;

/// Prints one update line per source and returns `0` when malformed sources abort it.
#[native(method)]
pub fn update(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(access) = access(vm, this) else {
        return Ok(NativeOutput::ready(if matches!(this, Value::Map(_)) {
            Value::Null
        } else {
            Value::FALSE
        }));
    };
    let message = AptClientUpdate {
        program: program(vm)?,
        apt_client: access.reference(),
    };

    Ok(settle(
        server(vm)?.send_apt_client_update(&message),
        |_vm, update| {
            let update = update.map_err(|failure| raise("update", failure))?;

            Ok::<_, IntrinsicError>(update.result)
        },
    ))
}
