use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::{is_admin, number};
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::AdminSeed;

/// Returns the world seed.
#[native(method)]
pub fn seed(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    if !is_admin(vm, this, "seed") {
        return Ok(NativeOutput::ready(Value::Null));
    }

    Ok(respond(
        vm,
        server(vm)?.send_admin_seed(&AdminSeed {
            program: program(vm)?,
        }),
        |_, seed| number(seed),
    ))
}
