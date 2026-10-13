use greybel_interpreter::{Value, Vm, native};

use super::port;
use crate::error::IntrinsicError;

#[native(method)]
pub fn get_lan_ip(vm: &mut Vm, this: &Value) -> Result<Option<String>, IntrinsicError> {
    Ok(port(vm, this)?.map(|port| port.lan_ip.clone()))
}
