use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};
use greyhack_behaviour::check_new_name;

use super::file;
use crate::error::IntrinsicError;
use crate::game::client::login_encryption::LoginEncryption;
use crate::game::client::server::{program, refuse, respond, server};
use crate::protocol::FileRename;

/// Renames the file; returns `""` on success or the failure reason.
#[native(method)]
pub fn rename(
    vm: &mut Vm,
    this: &Value,
    #[default("")]
    #[name = "newName"]
    new_name: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    LoginEncryption::deny_in(vm, "rename")?;

    if new_name.is_null() {
        return Ok(NativeOutput::ready(false));
    }

    let name = vm.heap().display(new_name).to_string();
    let Some(file) = file(vm, this, "rename") else {
        return Ok(NativeOutput::ready(false));
    };

    if let Err(refusal) = check_new_name(&name) {
        return refuse("rename", &refusal);
    }

    let message = FileRename {
        program: program(vm)?,
        file: file.reference(),
        name: name.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_file_rename(&message),
        |_, change| change.response("rename"),
    ))
}
