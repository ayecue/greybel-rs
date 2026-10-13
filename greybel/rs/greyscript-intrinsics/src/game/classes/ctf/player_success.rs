use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::ctf;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::CtfPlayerSuccess;

#[native(method)]
pub fn player_success(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(ctf) = ctf(vm, this, "player_success") else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    Ok(respond(
        vm,
        server(vm)?.send_ctf_player_success(&CtfPlayerSuccess {
            program: program(vm)?,
            ctf,
        }),
        |_, success| success,
    ))
}
