use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::access;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::AptClientAddRepo;

/// Registers an IP repository on `port` in `sources.txt`.
#[native(method)]
pub fn add_repo(
    vm: &mut Vm,
    this: &Value,
    repository: &Value,
    #[default(1542)] port: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let (Some(access), Some(repository), Some(port)) = (
        access(vm, this),
        repository.as_str(),
        port.as_int().and_then(|port| u16::try_from(port).ok()),
    ) else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    let message = AptClientAddRepo {
        program: program(vm)?,
        apt_client: access.reference(),
        repository: repository.into(),
        port,
    };

    Ok(respond(
        vm,
        server(vm)?.send_apt_client_add_repo(&message),
        |_, outcome| outcome.into_value_or_empty("add_repo"),
    ))
}
