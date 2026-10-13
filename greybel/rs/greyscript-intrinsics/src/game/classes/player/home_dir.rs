use greybel_interpreter::{Vm, native};
use greyhack_behaviour::path;

use crate::error::IntrinsicError;
use crate::game::game;

#[native]
pub fn home_dir(vm: &mut Vm) -> Result<String, IntrinsicError> {
    Ok(path::home(&game(vm)?.session.user))
}
