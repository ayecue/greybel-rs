use greybel_interpreter::{Value, Vm, native};

use super::is_admin;
use crate::error::IntrinsicError;
use crate::game::classes::admin_network;

/// Returns a handle to the network at `publicIp` without resolving it.
/// Operations on a missing network fail normally.
#[native(method)]
pub fn network(
    vm: &mut Vm,
    this: &Value,
    #[name = "publicIp"] public_ip: &Value,
) -> Result<Value, IntrinsicError> {
    const NAME: &str = "network";

    if !is_admin(vm, this, NAME) {
        return Ok(Value::Null);
    }

    let Some(public_ip) = public_ip.as_str() else {
        return Err(IntrinsicError::InvalidArguments(NAME));
    };
    let public_ip = public_ip.to_owned();

    Ok(admin_network::create(vm, public_ip))
}
