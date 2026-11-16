use greybel_interpreter::{NativeFuture, Value, Vm, native};

use crate::error::IntrinsicError;
use crate::game::client::object;

/// Returns a library object from the session computer.
#[native]
pub fn include_lib(
    vm: &mut Vm,
    #[name = "libPath"] path: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let path = match path.as_str() {
        Some(path) if !path.is_empty() => path,
        _ => return Err(IntrinsicError::InvalidArguments("include_lib")),
    };

    object::from_include(vm, path)
}
