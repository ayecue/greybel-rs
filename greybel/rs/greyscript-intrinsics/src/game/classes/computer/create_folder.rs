use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};
use greyhack_behaviour::{check_folder_name, path};

use super::computer;
use crate::error::IntrinsicError;
use crate::game::client::login_encryption::LoginEncryption;
use crate::game::client::server::{program, refuse, respond, server};
use crate::game::game;
use crate::protocol::ComputerCreateFolder;

/// Creates a folder, resolving relative paths from the current folder.
#[native(method)]
pub fn create_folder(
    vm: &mut Vm,
    this: &Value,
    path: &Value,
    #[default("newFolder")]
    #[name = "nameFolder"]
    name: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    LoginEncryption::deny_in(vm, "create_folder")?;

    let (Some(folder), Some(name)) = (path.as_str(), name.as_str()) else {
        return Err(IntrinsicError::InvalidArguments("create_folder"));
    };

    if let Err(refusal) = check_folder_name(name) {
        return refuse("create_folder", &refusal);
    }

    let Some(access) = computer(vm, this, "create_folder") else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    if path::has_too_many_folders(folder) {
        return Ok(NativeOutput::ready("create_folder: path too large"));
    }

    if !path::is_valid(folder) {
        return Ok(NativeOutput::ready("Error: invalid path"));
    }

    let folder = path::resolve(folder, &game(vm)?.session.current_path);
    let message = ComputerCreateFolder {
        program: program(vm)?,
        computer: access.reference(),
        folder: folder.into(),
        name: name.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_computer_create_folder(&message),
        |_, outcome| outcome.response("create_folder"),
    ))
}
