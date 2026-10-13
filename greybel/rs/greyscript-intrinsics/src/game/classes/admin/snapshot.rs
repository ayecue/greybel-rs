use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::is_admin;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::AdminSnapshot;

/// Immediately writes the world to the game-server database.
/// Returns `true` on success; returns `false` if no database exists or the write fails.
#[native(method)]
pub fn snapshot(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    if !is_admin(vm, this, "snapshot") {
        return Ok(NativeOutput::ready(Value::Null));
    }

    Ok(respond(
        vm,
        server(vm)?.send_admin_snapshot(&AdminSnapshot {
            program: program(vm)?,
        }),
        |_, written: bool| written,
    ))
}
