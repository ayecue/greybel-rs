use greybel_interpreter::{Value, Vm, native};

use crate::error::IntrinsicError;
use crate::game::game;

#[native(method)]
pub fn launch_path(vm: &mut Vm, _this: &Value) -> Result<String, IntrinsicError> {
    Ok(game(vm)?.session.launch_path.clone())
}
