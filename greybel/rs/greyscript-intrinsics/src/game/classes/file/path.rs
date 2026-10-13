use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::file;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::FilePath;

/// Returns the file path; `symlinkOrigPath` selects a symlink's target path.
#[native(method)]
pub fn path(
    vm: &mut Vm,
    this: &Value,
    #[default(0)]
    #[name = "symlinkOrigPath"]
    original: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let (Some(original), Some(file)) = (original.as_number(), file(vm, this, "path")) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = FilePath {
        program: program(vm)?,
        file: file.reference(),
        original: original != 0.0,
    };

    Ok(respond(
        vm,
        server(vm)?.send_file_path(&message),
        |_, path| path,
    ))
}
