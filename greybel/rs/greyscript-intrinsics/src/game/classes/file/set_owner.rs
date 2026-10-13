use greybel_interpreter::{NativeFuture, Value, Vm, native};
use greyhack_behaviour::FileOwnership;

use super::change_ownership;
use crate::error::IntrinsicError;

#[native(method)]
pub fn set_owner(
    vm: &mut Vm,
    this: &Value,
    #[default("")] owner: &Value,
    #[default(0)]
    #[name = "isRecursive"]
    recursive: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    change_ownership(vm, this, FileOwnership::Owner, owner, recursive)
}
