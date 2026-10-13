use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::access;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::AptClientShow;

/// Returns every package offered by repository `repository`.
#[native(method)]
pub fn show(vm: &mut Vm, this: &Value, repository: &Value) -> Result<NativeFuture, IntrinsicError> {
    let (Some(access), Some(repository)) = (access(vm, this), repository.as_str()) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = AptClientShow {
        program: program(vm)?,
        apt_client: access.reference(),
        repository: repository.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_apt_client_show(&message),
        |_, packages| packages.response("show"),
    ))
}
