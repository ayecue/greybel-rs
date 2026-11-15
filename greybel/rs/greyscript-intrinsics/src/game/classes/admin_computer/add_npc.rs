use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::address;
use crate::error::IntrinsicError;
use crate::game::client::object::ComputerAddress;
use crate::game::client::server::{program, respond, server};
use crate::protocol::AdminAddNpc;

/// Adds an offline NPC who logs into this computer as `user`, with the first login after `delay` seconds.
#[native(method)]
pub fn add_npc(
    vm: &mut Vm,
    this: &Value,
    user: &Value,
    delay: &Value,
    #[default("npc")] name: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    const NAME: &str = "add_npc";

    let Some(ComputerAddress { public_ip, lan_ip }) = address(vm, this, NAME) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let (Some(user), Some(delay), Some(name)) = (user.as_str(), delay.as_int(), name.as_str())
    else {
        return Err(IntrinsicError::InvalidArguments(NAME));
    };
    let message = AdminAddNpc {
        program: program(vm)?,
        public_ip: public_ip.into(),
        lan_ip: lan_ip.into(),
        user: user.into(),
        delay: u64::try_from(delay.max(0)).expect("the clamped delay is nonnegative"),
        name: name.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_add_npc(&message),
        |_, id: Option<u32>| id.map(f64::from),
    ))
}
