use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};
use greyhack_behaviour::{check_file_name, path};

use super::computer;
use crate::error::IntrinsicError;
use crate::game::client::login_encryption::LoginEncryption;
use crate::game::client::server::{program, refuse, respond, server};
use crate::protocol::ComputerTouch;

/// Creates an empty text file without resolving the folder path against the current folder.
#[native(method)]
pub fn touch(
    vm: &mut Vm,
    this: &Value,
    path: &Value,
    #[name = "nameFile"] name: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    LoginEncryption::deny_in(vm, "touch")?;

    let Some(folder) = path.as_str() else {
        return Ok(NativeOutput::ready("Error: invalid path"));
    };
    let Some(name) = name.as_str() else {
        return Ok(NativeOutput::ready("Error: nameFile must be string"));
    };

    if let Err(refusal) = check_file_name(name) {
        return refuse("touch", refusal);
    }

    let Some(access) = computer(vm, this, "touch") else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    if path::has_too_many_folders(folder) {
        return Ok(NativeOutput::ready("touch: path too large"));
    }

    if !path::is_valid(folder) {
        return Ok(NativeOutput::ready("Error: invalid path"));
    }

    let message = ComputerTouch {
        program: program(vm)?,
        computer: access.reference(),
        folder: folder.into(),
        name: name.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_computer_touch(&message),
        |_, outcome| outcome.response("touch"),
    ))
}
