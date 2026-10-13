use greybel_interpreter::{Vm, native};

use crate::error::IntrinsicError;
use crate::game::game;

#[native]
pub fn program_path(vm: &mut Vm) -> Result<String, IntrinsicError> {
    Ok(game(vm)?.session.program_path.clone())
}
