use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::computer;
use crate::error::IntrinsicError;
use crate::game::client::login_encryption::LoginEncryption;
use crate::game::client::server::{program, respond, server};
use crate::protocol::ComputerReboot;

#[native(method)]
pub fn reboot(
    vm: &mut Vm,
    this: &Value,
    #[name = "safeMode"]
    #[default(0)]
    safe_mode: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let Some(access) = computer(vm, this, "reboot") else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let Some(safe_mode) = safe_mode.as_number() else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    LoginEncryption::deny_in(vm, "reboot")?;

    let message = ComputerReboot {
        program: program(vm)?,
        computer: access.reference(),
        safe_mode: safe_mode != 0.0,
    };

    Ok(respond(
        vm,
        server(vm)?.send_computer_reboot(&message),
        |_, outcome| outcome.response("reboot"),
    ))
}
