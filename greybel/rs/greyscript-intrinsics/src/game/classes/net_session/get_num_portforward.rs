use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::session;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::NetSessionGetNumPortforward;

#[native(method)]
pub fn get_num_portforward(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(target) = session(vm, this) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = NetSessionGetNumPortforward {
        program: program(vm)?,
        net_session: target.reference(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_net_session_get_num_portforward(&message),
        |_, count| count,
    ))
}
