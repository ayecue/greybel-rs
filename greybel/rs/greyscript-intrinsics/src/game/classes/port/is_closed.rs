use greybel_interpreter::{Value, Vm, native};

use super::port;
use crate::error::IntrinsicError;

#[native(method)]
pub fn is_closed(vm: &mut Vm, this: &Value) -> Result<Option<bool>, IntrinsicError> {
    Ok(port(vm, this)?.map(|port| port.closed))
}
