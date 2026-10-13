use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};
use greyhack_behaviour::check_target_name;

use super::file;
use crate::error::IntrinsicError;
use crate::game::client::login_encryption::LoginEncryption;
use crate::game::client::server::{program, raise, refuse, server, settle};
use crate::protocol::FileMove;

/// Moves the file into folder `path` as `newName` and updates this object to reference the destination.
#[native(method)]
pub fn r#move(
    vm: &mut Vm,
    this: &Value,
    #[default("")] path: &Value,
    #[default("")]
    #[name = "newName"]
    new_name: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    LoginEncryption::deny_in(vm, "move")?;

    let (Some(folder), Some(name)) = (path.as_str(), new_name.as_str()) else {
        return Err(IntrinsicError::InvalidArguments("move"));
    };
    let Some(file) = file(vm, this, "move") else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    if let Err(refusal) = check_target_name(name) {
        return refuse("move", refusal);
    }

    let message = FileMove {
        program: program(vm)?,
        file: file.reference(),
        folder: folder.into(),
        name: name.into(),
    };

    Ok(settle(
        server(vm)?.send_file_move(&message),
        |_, outcome| {
            outcome
                .map_err(|failure| raise("move", failure))?
                .response("move")
        },
    ))
}
