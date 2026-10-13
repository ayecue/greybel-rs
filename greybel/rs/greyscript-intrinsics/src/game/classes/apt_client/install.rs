use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::access;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, raise, server, settle};
use crate::game::send_output;
use crate::protocol::{AptClientInstall, Outcome};

/// Installs `package` into `custom_path` or the default folder.
/// Returns `true` on success or Grey Hack's failure message.
#[native(method)]
pub fn install(
    vm: &mut Vm,
    this: &Value,
    package: &Value,
    #[default("")] custom_path: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let (Some(access), Some(package), Some(custom_path)) =
        (access(vm, this), package.as_str(), custom_path.as_str())
    else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = AptClientInstall {
        program: program(vm)?,
        apt_client: access.reference(),
        package: package.into(),
        folder: custom_path.into(),
    };

    Ok(settle(
        server(vm)?.send_apt_client_install(&message),
        |vm, outcome| {
            // Grey Hack flushes output when the transfer starts; the transfer returns true.
            if matches!(outcome, Ok(Outcome::True)) {
                send_output(vm);
            }

            outcome
                .map_err(|failure| raise("install", failure))?
                .response("install")
        },
    ))
}
