use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::service;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::ServiceInstallService;

/// Installs the service; requires root (`install_service`).
#[native(method)]
pub fn install_service(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(service) = service(vm, this) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = ServiceInstallService {
        program: program(vm)?,
        service,
    };

    Ok(respond(
        vm,
        server(vm)?.send_service_install_service(&message),
        |_, outcome| outcome.response("install_service"),
    ))
}
