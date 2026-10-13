use greybel_interpreter::{Value, Vm, native};
use greyhack_behaviour::path::{self, MAX_BASE_PATH, MAX_PATH};

use crate::error::IntrinsicError;
use crate::game::game;

#[native]
pub fn get_abs_path(
    vm: &mut Vm,
    path: &Value,
    #[default("")]
    #[name = "basePath"]
    base: &Value,
) -> Result<String, IntrinsicError> {
    let (Some(path), Some(base)) = (path.as_str(), base.as_str()) else {
        return Err(IntrinsicError::InvalidArguments("get_abs_path"));
    };

    if vm.heap().text_len(path) > MAX_PATH {
        return Err(IntrinsicError::PathTooLarge);
    }

    if vm.heap().text_len(base) > MAX_BASE_PATH {
        return Err(IntrinsicError::BasePathTooLarge);
    }

    let session = &game(vm)?.session;
    let current = if base.is_empty() {
        &session.current_path
    } else {
        base
    };

    Ok(path::absolute(path, current, &session.user))
}
