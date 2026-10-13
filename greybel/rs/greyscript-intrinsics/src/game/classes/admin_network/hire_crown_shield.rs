use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::public_ip;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::AdminHireCrownShield;

/// Hires CrownShield for guild `guild` at this network, as the guild's owner does in the game's interface
/// (`NetworkHelperServer.ActivateCrownShieldProtection`). The price is not charged. Returns true or the failure text.
#[native(method)]
pub fn hire_crown_shield(
    vm: &mut Vm,
    this: &Value,
    guild: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    const NAME: &str = "hire_crown_shield";

    let Some(public_ip) = public_ip(vm, this, NAME) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let Some(guild) = guild.as_str() else {
        return Err(IntrinsicError::InvalidArguments(NAME));
    };
    let message = AdminHireCrownShield {
        program: program(vm)?,
        guild: guild.into(),
        public_ip: public_ip.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_hire_crown_shield(&message),
        |_, outcome| outcome.response(NAME),
    ))
}
