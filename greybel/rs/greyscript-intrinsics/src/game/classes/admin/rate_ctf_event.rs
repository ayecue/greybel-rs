use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::is_admin;
use crate::error::IntrinsicError;
use crate::game::client::failure::failure_value;
use crate::game::client::server::{program, respond, server};
use crate::protocol::{AdminRateCtfEvent, CtfRating};

/// Rates an event as the current sandbox player and returns both rewards.
#[native(method)]
#[expect(
    clippy::cast_precision_loss,
    reason = "GreyScript exposes CTF reward amounts as f64 numbers"
)]
pub fn rate_ctf_event(
    vm: &mut Vm,
    this: &Value,
    name: &Value,
    quality: &Value,
    difficulty: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    const NAME: &str = "rate_ctf_event";

    if !is_admin(vm, this, NAME) {
        return Ok(NativeOutput::ready(Value::Null));
    }

    let Some(name) = name.as_str() else {
        return Err(IntrinsicError::InvalidArguments(NAME));
    };
    let (Some(quality), Some(difficulty)) = (quality.as_int(), difficulty.as_int()) else {
        return Err(IntrinsicError::InvalidArguments(NAME));
    };
    let message = AdminRateCtfEvent {
        program: program(vm)?,
        name: name.into(),
        quality: i64::from(quality),
        difficulty: i64::from(difficulty),
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_rate_ctf_event(&message),
        |vm, outcome| {
            Ok::<_, IntrinsicError>(match outcome {
                CtfRating::Rewards {
                    participant,
                    creator,
                } => Value::List(
                    vm.heap_mut()
                        .add_list(vec![(participant as f64).into(), (creator as f64).into()]),
                ),
                CtfRating::Failure(failure) => failure_value(NAME, &failure)?,
            })
        },
    ))
}
