use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::shell;
use crate::error::IntrinsicError;
use crate::game::client::login_encryption::LoginEncryption;
use crate::game::client::server::{program, respond, server};
use crate::game::send_output;
use crate::protocol::{Outcome, ShellScp};

/// Copies a file between local and remote shells.
#[native(method)]
pub fn scp(
    vm: &mut Vm,
    this: &Value,
    path_orig: &Value,
    path_dest: &Value,
    remote_shell: &Value,
    #[default(0)] is_upload: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let (Some(from), Some(source), Some(destination), Some(is_upload)) = (
        shell(vm, this, "scp"),
        path_orig.as_str(),
        path_dest.as_str(),
        is_upload.as_number(),
    ) else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    if source.is_empty() || destination.is_empty() {
        return Err(IntrinsicError::InvalidArguments("scp"));
    }

    LoginEncryption::deny_in(vm, "scp")?;

    let message = ShellScp {
        program: program(vm)?,
        from: from.reference(),
        to: shell(vm, remote_shell, "scp").map(|to| to.reference()),
        source: source.to_owned().into(),
        destination: destination.to_owned().into(),
        upload: is_upload != 0.0,
    };

    Ok(respond(
        vm,
        server(vm)?.send_shell_scp(&message),
        |vm, outcome| {
            // Grey Hack flushes output when the transfer starts; the transfer returns true.
            if matches!(outcome, Outcome::True) {
                send_output(vm);
            }

            outcome.response("scp")
        },
    ))
}
