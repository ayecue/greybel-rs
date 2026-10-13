use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::public_ip;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::AdminSetAdminRestore;

/// Turns Admin Restore off (`enabled` 0) or on for this network's company as guild `guild`, as its company
/// management window does (`NetworkHelperServer.AdminCompanySetAdminRestoreServerRpc`). Returns true or the failure
/// text.
#[native(method)]
pub fn set_admin_restore(
    vm: &mut Vm,
    this: &Value,
    guild: &Value,
    enabled: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    const NAME: &str = "set_admin_restore";

    let Some(public_ip) = public_ip(vm, this, NAME) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let (Some(guild), Some(on)) = (guild.as_str(), enabled.as_number()) else {
        return Err(IntrinsicError::InvalidArguments(NAME));
    };
    let message = AdminSetAdminRestore {
        program: program(vm)?,
        guild: guild.into(),
        public_ip: public_ip.into(),
        enabled: on != 0.0,
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_set_admin_restore(&message),
        |_, outcome| outcome.response(NAME),
    ))
}
