use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::access;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::AptClientCheckUpgrade;

/// Tests whether configured repositories offer a newer version of the library at `filePath`.
#[native(method)]
pub fn check_upgrade(
    vm: &mut Vm,
    this: &Value,
    #[name = "filePath"] path_arg: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let (Some(access), Some(path_arg)) = (access(vm, this), path_arg.as_str()) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = AptClientCheckUpgrade {
        program: program(vm)?,
        apt_client: access.reference(),
        path: path_arg.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_apt_client_check_upgrade(&message),
        |_, newer| newer.response("check_upgrade"),
    ))
}
