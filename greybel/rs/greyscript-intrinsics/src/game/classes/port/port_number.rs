use greybel_interpreter::{Value, Vm, native};

use super::port;
use crate::error::IntrinsicError;

#[native(method)]
pub fn port_number(vm: &mut Vm, this: &Value) -> Result<Option<u16>, IntrinsicError> {
    Ok(port(vm, this)?.map(|port| port.number))
}
