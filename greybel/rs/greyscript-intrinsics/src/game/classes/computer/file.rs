use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};
use greyhack_behaviour::path;

use super::computer;
use crate::error::IntrinsicError;
use crate::game::client::object;
use crate::game::client::object::FileObject;
use crate::game::client::server::{program, respond, server};
use crate::game::game;
use crate::protocol::ComputerFile;

/// Implements `File(path)`, resolving relative paths from the current folder.
/// Symlinks resolve to the link itself rather than its target.
#[native(method, label = "File")]
pub fn file(vm: &mut Vm, this: &Value, path: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(access) = computer(vm, this, "File") else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let path = match path.as_str() {
        Some(path) if !path.is_empty() => path.to_owned(),
        _ => return Err(IntrinsicError::InvalidArguments("File")),
    };
    let path = path::resolve(&path, &game(vm)?.session.current_path);
    let message = ComputerFile {
        program: program(vm)?,
        computer: access.reference(),
        path: path.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_computer_file(&message),
        move |vm, file| {
            let Some(file) = file else {
                return Ok(Value::Null);
            };
            let file = FileObject {
                object: object::handle(vm, file)?,
                ftp: access.ftp,
            };

            Ok::<_, IntrinsicError>(object::create_file(vm, file))
        },
    ))
}
