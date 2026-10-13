use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::access;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::AptClientSearch;

/// Returns packages matching `search` across configured repositories.
#[native(method)]
pub fn search(
    vm: &mut Vm,
    this: &Value,
    #[name = "search"] term: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let (Some(access), Some(term)) = (access(vm, this), term.as_str()) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = AptClientSearch {
        program: program(vm)?,
        apt_client: access.reference(),
        term: term.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_apt_client_search(&message),
        |_, packages| packages.response("search"),
    ))
}
