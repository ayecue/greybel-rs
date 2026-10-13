use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::address;
use crate::error::IntrinsicError;
use crate::game::client::object::ComputerAddress;
use crate::game::client::server::{program, respond, server};
use crate::protocol::AdminSetLibraryVersion;

/// Updates the library version at `path` on this computer.
#[native(method)]
pub fn set_library_version(
    vm: &mut Vm,
    this: &Value,
    path: &Value,
    version: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    const NAME: &str = "set_library_version";
    let Some(ComputerAddress { public_ip, lan_ip }) = address(vm, this, NAME) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let (Some(path), Some(version)) = (path.as_str(), version.as_str()) else {
        return Err(IntrinsicError::InvalidArguments(NAME));
    };
    let message = AdminSetLibraryVersion {
        program: program(vm)?,
        public_ip: public_ip.into(),
        lan_ip: lan_ip.into(),
        path: path.into(),
        version: version.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_set_library_version(&message),
        |_, changed: bool| changed,
    ))
}
