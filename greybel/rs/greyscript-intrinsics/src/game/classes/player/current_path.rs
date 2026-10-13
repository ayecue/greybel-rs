use greybel_interpreter::{Value, Vm, native};

use crate::error::IntrinsicError;
use crate::game::game;

/// Returns the program's current folder; Grey Hack declares this global with a `self` parameter.
#[native(method)]
pub fn current_path(vm: &mut Vm, _this: &Value) -> Result<String, IntrinsicError> {
    Ok(game(vm)?.session.current_path.clone())
}
