use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::service;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::ServiceStopService;

/// Closes the service port and kills its daemon; requires root.
#[native(method)]
pub fn stop_service(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(service) = service(vm, this) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = ServiceStopService {
        program: program(vm)?,
        service,
    };

    Ok(respond(
        vm,
        server(vm)?.send_service_stop_service(&message),
        |_, outcome| outcome.response("stop_service"),
    ))
}
