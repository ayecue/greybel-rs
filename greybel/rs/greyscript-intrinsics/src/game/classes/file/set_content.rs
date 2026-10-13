use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};
use greyhack_behaviour::check_content;

use super::file;
use crate::error::IntrinsicError;
use crate::game::client::login_encryption::LoginEncryption;
use crate::game::client::server::{program, refuse, respond, server};
use crate::protocol::FileSetContent;

#[native(method)]
pub fn set_content(
    vm: &mut Vm,
    this: &Value,
    #[default("")] content: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let Some(file) = file(vm, this, "set_content").filter(|_| !content.is_null()) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let content = vm.heap().display(content).to_string();

    if let Err(refusal) = check_content(&content) {
        return refuse("set_content", refusal);
    }

    LoginEncryption::deny_in(vm, "set_content")?;

    let message = FileSetContent {
        program: program(vm)?,
        file: file.reference(),
        content: content.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_file_set_content(&message),
        |_, outcome| outcome.response("set_content"),
    ))
}
