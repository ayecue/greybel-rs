use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};
use greyhack_behaviour::path;

use crate::error::IntrinsicError;
use crate::game::classes::admin;
use crate::game::client::object;
use crate::game::game;

/// Returns a library object from the session computer.
/// `admin.so` is always available unless explicitly disabled.
#[native]
pub fn include_lib(
    vm: &mut Vm,
    #[name = "libPath"] path: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let path = match path.as_str() {
        Some(path) if !path.is_empty() => path,
        _ => return Err(IntrinsicError::InvalidArguments("include_lib")),
    };

    if path::file_name(path) == admin::LIBRARY && game(vm)?.admin {
        return Ok(NativeOutput::ready(admin::create(vm)));
    }

    object::from_include(vm, path)
}
