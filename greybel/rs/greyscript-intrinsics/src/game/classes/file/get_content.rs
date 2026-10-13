use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::file;
use crate::error::IntrinsicError;
use crate::game::client::login_encryption::LoginEncryption;
use crate::game::client::server::{program, respond, server};
use crate::protocol::FileGetContent;

#[native(method)]
pub fn get_content(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(file) = file(vm, this, "get_content") else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    LoginEncryption::deny_in(vm, "get_content")?;

    let message = FileGetContent {
        program: program(vm)?,
        file: file.reference(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_file_get_content(&message),
        |_, content| content.response("get_content"),
    ))
}
