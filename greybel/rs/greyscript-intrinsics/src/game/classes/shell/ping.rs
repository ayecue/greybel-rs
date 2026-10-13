use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};
use greyhack_behaviour::is_valid_ip;

use super::shell;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::ShellPing;

/// Tests whether `ipAddress` responds from the shell's network.
#[native(method)]
pub fn ping(
    vm: &mut Vm,
    this: &Value,
    #[name = "ipAddress"] address: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let (Some(from), Some(address)) = (shell(vm, this, "ping"), address.as_str()) else {
        return Err(IntrinsicError::InvalidArguments("ping"));
    };

    if !is_valid_ip(address) {
        return Ok(NativeOutput::ready("ping: invalid ip address"));
    }

    let message = ShellPing {
        program: program(vm)?,
        shell: from.reference(),
        address: address.to_owned().into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_shell_ping(&message),
        |_, responds| responds,
    ))
}
