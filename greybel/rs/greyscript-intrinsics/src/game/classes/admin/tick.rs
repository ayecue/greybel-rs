use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::{is_admin, number};
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::AdminTick;

/// Advances the deterministic world clock and returns its elapsed seconds.
#[native(method)]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "The admin tick protocol stores a nonnegative whole-second duration"
)]
pub fn tick(vm: &mut Vm, this: &Value, seconds: &Value) -> Result<NativeFuture, IntrinsicError> {
    if !is_admin(vm, this, "tick") {
        return Ok(NativeOutput::ready(Value::Null));
    }

    let Some(seconds) = seconds.as_number() else {
        return Err(IntrinsicError::InvalidArguments("tick"));
    };

    if seconds < 0.0 {
        return Err(IntrinsicError::InvalidArguments("tick"));
    }

    let message = AdminTick {
        program: program(vm)?,
        seconds: seconds as u64,
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_tick(&message),
        |_, at| number(at),
    ))
}
