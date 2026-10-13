use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::address;
use crate::error::IntrinsicError;
use crate::game::client::object;
use crate::game::client::object::{Access, ComputerAddress};
use crate::game::client::server::{program, respond, server};
use crate::protocol::AdminShell;

/// Returns a passwordless shell on this computer as `user`.
#[native(method)]
pub fn shell(
    vm: &mut Vm,
    this: &Value,
    #[default("root")] user: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    const NAME: &str = "shell";

    let Some(ComputerAddress { public_ip, lan_ip }) = address(vm, this, NAME) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let Some(user) = user.as_str() else {
        return Err(IntrinsicError::InvalidArguments(NAME));
    };
    let message = AdminShell {
        program: program(vm)?,
        public_ip: public_ip.into(),
        lan_ip: lan_ip.into(),
        user: user.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_shell(&message),
        |vm, shell| {
            let Some(shell) = shell else {
                return Ok(Value::Null);
            };
            let shell = Access::new(object::handle(vm, shell)?);

            Ok::<_, IntrinsicError>(object::create_shell(vm, shell))
        },
    ))
}
