use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::address;
use crate::error::IntrinsicError;
use crate::game::client::object::ComputerAddress;
use crate::game::client::server::{program, respond, server};
use crate::protocol::AdminNpcLogin;

/// Triggers social-engineering login for NPC `user` on this computer's SSH/FTP service.
/// Exposes its credentials to sniffers along the route (`Persona.ProcesaIngenieraSocial`).
#[native(method)]
pub fn npc_login(vm: &mut Vm, this: &Value, user: &Value) -> Result<NativeFuture, IntrinsicError> {
    const NAME: &str = "npc_login";
    let Some(ComputerAddress { public_ip, lan_ip }) = address(vm, this, NAME) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let Some(user) = user.as_str() else {
        return Err(IntrinsicError::InvalidArguments(NAME));
    };
    let message = AdminNpcLogin {
        program: program(vm)?,
        address: public_ip.into(),
        lan_ip: lan_ip.into(),
        user: user.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_npc_login(&message),
        |_, done: bool| done,
    ))
}
