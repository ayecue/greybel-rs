use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::session;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::NetSessionIsRootActiveUser;

#[native(method)]
pub fn is_root_active_user(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(target) = session(vm, this) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = NetSessionIsRootActiveUser {
        program: program(vm)?,
        net_session: target.reference(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_net_session_is_root_active_user(&message),
        |_, active| active,
    ))
}
