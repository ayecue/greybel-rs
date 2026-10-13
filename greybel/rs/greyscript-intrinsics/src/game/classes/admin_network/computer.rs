use greybel_interpreter::{Value, Vm, native};

use super::public_ip;
use crate::error::IntrinsicError;
use crate::game::classes::admin_computer;
use crate::game::client::object::ComputerAddress;

/// Returns a handle to `lanIp` in this network without resolving it.
/// Like network handles, operations on a missing computer fail normally.
#[native(method)]
pub fn computer(
    vm: &mut Vm,
    this: &Value,
    #[name = "lanIp"] lan_ip: &Value,
) -> Result<Value, IntrinsicError> {
    const NAME: &str = "computer";

    let Some(public_ip) = public_ip(vm, this, NAME) else {
        return Ok(Value::Null);
    };
    let Some(lan_ip) = lan_ip.as_str() else {
        return Err(IntrinsicError::InvalidArguments(NAME));
    };
    let address = ComputerAddress {
        public_ip,
        lan_ip: lan_ip.to_owned(),
    };

    Ok(admin_computer::create(vm, address))
}
