use greybel_interpreter::{Vm, native};

use crate::error::IntrinsicError;
use crate::game::game;

#[native]
pub fn active_user(vm: &mut Vm) -> Result<String, IntrinsicError> {
    Ok(game(vm)?.session.user.clone())
}
