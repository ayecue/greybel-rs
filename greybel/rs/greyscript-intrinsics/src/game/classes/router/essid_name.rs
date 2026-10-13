use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::router_id;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::RouterEssidName;

#[native(method)]
pub fn essid_name(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(router) = router_id(vm, this) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = RouterEssidName {
        program: program(vm)?,
        router,
    };

    Ok(respond(
        vm,
        server(vm)?.send_router_essid_name(&message),
        |_, essid| essid,
    ))
}
