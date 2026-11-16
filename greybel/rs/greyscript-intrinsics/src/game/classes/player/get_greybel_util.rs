use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use crate::error::IntrinsicError;
use crate::game::classes::admin;
use crate::game::game;

/// Returns Greybel's sandbox admin object when the host enables the utility.
#[native]
pub fn get_greybel_util(vm: &mut Vm) -> Result<NativeFuture, IntrinsicError> {
    if !game(vm)?.admin {
        return Ok(NativeOutput::ready(Value::Null));
    }

    Ok(NativeOutput::ready(admin::create(vm)))
}
