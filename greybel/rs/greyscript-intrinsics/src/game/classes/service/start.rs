use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::service;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::ServiceStartService;

/// Opens the service port and runs its daemon; requires root.
#[native(method)]
pub fn start_service(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(service) = service(vm, this) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = ServiceStartService {
        program: program(vm)?,
        service,
    };

    Ok(respond(
        vm,
        server(vm)?.send_service_start_service(&message),
        |_, outcome| outcome.response("start_service"),
    ))
}
