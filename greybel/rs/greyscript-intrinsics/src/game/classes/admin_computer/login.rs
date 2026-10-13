use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::address;
use crate::error::IntrinsicError;
use crate::game::client::object::ComputerAddress;
use crate::game::client::server::{program, respond, server};
use crate::protocol::AdminLogin;

/// Opens a session for `user` so active-user checks detect them; not available in Grey Hack.
#[native(method)]
pub fn login(vm: &mut Vm, this: &Value, user: &Value) -> Result<NativeFuture, IntrinsicError> {
    const NAME: &str = "login";

    let Some(ComputerAddress { public_ip, lan_ip }) = address(vm, this, NAME) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let Some(user) = user.as_str() else {
        return Err(IntrinsicError::InvalidArguments(NAME));
    };
    let message = AdminLogin {
        program: program(vm)?,
        public_ip: public_ip.into(),
        lan_ip: lan_ip.into(),
        user: user.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_login(&message),
        |_, opened: bool| opened,
    ))
}
