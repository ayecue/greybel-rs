use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::ctf;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::CtfGetDescription;

#[native(method)]
pub fn get_description(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(ctf) = ctf(vm, this, "get_description") else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    Ok(respond(
        vm,
        server(vm)?.send_ctf_get_description(&CtfGetDescription {
            program: program(vm)?,
            ctf,
        }),
        |_, description| description,
    ))
}
