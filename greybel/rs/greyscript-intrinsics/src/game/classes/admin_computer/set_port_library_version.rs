use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::address;
use crate::error::IntrinsicError;
use crate::game::client::object::ComputerAddress;
use crate::game::client::server::{program, respond, server};
use crate::protocol::AdminSetPortLibraryVersion;

/// Sets the library version behind this computer's `port`.
#[native(method)]
pub fn set_port_library_version(
    vm: &mut Vm,
    this: &Value,
    port: &Value,
    version: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    const NAME: &str = "set_port_library_version";
    let Some(ComputerAddress { public_ip, lan_ip }) = address(vm, this, NAME) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let (Some(port), Some(version)) = (port.as_int(), version.as_str()) else {
        return Err(IntrinsicError::InvalidArguments(NAME));
    };
    let Ok(port) = u16::try_from(port) else {
        return Err(IntrinsicError::InvalidArguments(NAME));
    };
    let message = AdminSetPortLibraryVersion {
        program: program(vm)?,
        public_ip: public_ip.into(),
        lan_ip: lan_ip.into(),
        port,
        version: version.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_set_port_library_version(&message),
        |_, changed: bool| changed,
    ))
}
