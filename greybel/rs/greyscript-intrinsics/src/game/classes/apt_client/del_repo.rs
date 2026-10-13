use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::access;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::AptClientDelRepo;

/// Deletes `repository` from `sources.txt`.
#[native(method)]
pub fn del_repo(
    vm: &mut Vm,
    this: &Value,
    repository: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let (Some(access), Some(repository)) = (access(vm, this), repository.as_str()) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = AptClientDelRepo {
        program: program(vm)?,
        apt_client: access.reference(),
        repository: repository.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_apt_client_del_repo(&message),
        |_, outcome| outcome.into_value_or_empty("del_repo"),
    ))
}
