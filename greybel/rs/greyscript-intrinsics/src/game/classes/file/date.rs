use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::file;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::FileDate;

/// Returns the file timestamp in `dd/MMM/yyyy HH:mm` format.
#[native(method)]
pub fn date(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(file) = file(vm, this, "date") else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = FileDate {
        program: program(vm)?,
        file: file.reference(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_file_date(&message),
        |_, date| date,
    ))
}
