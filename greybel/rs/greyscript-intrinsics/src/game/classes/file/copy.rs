use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};
use greyhack_behaviour::{check_target_name, path};

use super::file;
use crate::error::IntrinsicError;
use crate::game::client::login_encryption::LoginEncryption;
use crate::game::client::server::{program, refuse, respond, server};
use crate::game::send_output;
use crate::protocol::{FileCopy, Outcome};

/// Copies a file immediately instead of using Grey Hack's deferred download.
#[native(method)]
pub fn copy(
    vm: &mut Vm,
    this: &Value,
    #[default("")] path: &Value,
    #[default("")]
    #[name = "newName"]
    new_name: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    LoginEncryption::deny_in(vm, "copy")?;

    let (Some(folder), Some(name)) = (path.as_str(), new_name.as_str()) else {
        return Err(IntrinsicError::InvalidArguments("copy"));
    };
    let Some(file) = file(vm, this, "copy") else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    if let Err(refusal) = check_target_name(name) {
        return refuse("copy", &refusal);
    }

    if path::has_too_many_folders(folder) {
        return Err(IntrinsicError::TooManyFolders("copy"));
    }

    let message = FileCopy {
        program: program(vm)?,
        file: file.reference(),
        folder: folder.into(),
        name: name.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_file_copy(&message),
        |vm, outcome| {
            // Grey Hack flushes output when the transfer starts; the transfer returns true.
            if matches!(outcome, Outcome::True) {
                send_output(vm);
            }

            outcome.response("copy")
        },
    ))
}
