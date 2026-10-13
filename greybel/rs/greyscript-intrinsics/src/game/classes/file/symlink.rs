use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};
use greyhack_behaviour::check_target_name;

use super::file;
use crate::error::IntrinsicError;
use crate::game::client::login_encryption::LoginEncryption;
use crate::game::client::server::{program, raise, refuse, server, settle};
use crate::protocol::FileSymlink;

/// Creates a symlink named `newName` in folder `path`, targeting this file.
#[native(method)]
pub fn symlink(
    vm: &mut Vm,
    this: &Value,
    #[default("")] path: &Value,
    #[default("")]
    #[name = "newName"]
    new_name: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    LoginEncryption::deny_in(vm, "symlink")?;

    let (Some(folder), Some(name)) = (path.as_str(), new_name.as_str()) else {
        return Err(IntrinsicError::InvalidArguments("symlink"));
    };
    let Some(file) = file(vm, this, "symlink") else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    if let Err(refusal) = check_target_name(name) {
        return refuse("symlink", refusal);
    }

    let message = FileSymlink {
        program: program(vm)?,
        file: file.reference(),
        folder: folder.into(),
        name: name.into(),
    };

    Ok(settle(
        server(vm)?.send_file_symlink(&message),
        |_, outcome| {
            outcome
                .map_err(|failure| raise("symlink", failure))?
                .response("symlink")
        },
    ))
}
