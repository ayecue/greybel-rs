use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::ctf;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::CtfGetMailContent;

#[native(method)]
pub fn get_mail_content(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(ctf) = ctf(vm, this, "get_mail_content") else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    Ok(respond(
        vm,
        server(vm)?.send_ctf_get_mail_content(&CtfGetMailContent {
            program: program(vm)?,
            ctf,
        }),
        |_, content| content,
    ))
}
