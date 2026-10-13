use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::ctf;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::CtfGetCreatorName;

#[native(method)]
pub fn get_creator_name(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(ctf) = ctf(vm, this, "get_creator_name") else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    Ok(respond(
        vm,
        server(vm)?.send_ctf_get_creator_name(&CtfGetCreatorName {
            program: program(vm)?,
            ctf,
        }),
        |_, name| name,
    ))
}
