use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::public_ip;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::AdminFireNpc;

/// Fires this network's NPC `user` like the company employees website (`NetworkLan.FireNpc`).
/// The NPC dies after five minutes and vacates the position. Returns true or Grey Hack's failure reason.
#[native(method)]
pub fn fire_npc(vm: &mut Vm, this: &Value, user: &Value) -> Result<NativeFuture, IntrinsicError> {
    const NAME: &str = "fire_npc";

    let Some(public_ip) = public_ip(vm, this, NAME) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let Some(user) = user.as_str() else {
        return Err(IntrinsicError::InvalidArguments(NAME));
    };
    let message = AdminFireNpc {
        program: program(vm)?,
        public_ip: public_ip.into(),
        user: user.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_fire_npc(&message),
        |_, outcome| outcome.response(NAME),
    ))
}
