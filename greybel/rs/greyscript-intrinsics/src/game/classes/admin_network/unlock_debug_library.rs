use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::public_ip;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::AdminUnlockDebugLibrary;

#[native(method)]
pub fn unlock_debug_library(
    vm: &mut Vm,
    this: &Value,
    lan_ip: &Value,
    path: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    const NAME: &str = "unlock_debug_library";

    let Some(public_ip) = public_ip(vm, this, NAME) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let (Some(lan_ip), Some(path)) = (lan_ip.as_str(), path.as_str()) else {
        return Err(IntrinsicError::InvalidArguments(NAME));
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_unlock_debug_library(&AdminUnlockDebugLibrary {
            program: program(vm)?,
            public_ip: public_ip.into(),
            lan_ip: lan_ip.into(),
            path: path.into(),
        }),
        |_, unlocked: bool| Ok::<_, IntrinsicError>(Value::from(unlocked)),
    ))
}
