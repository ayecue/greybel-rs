use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::session;
use crate::error::IntrinsicError;
use crate::game::client::object;
use crate::game::client::object::{GameObject, Kind};
use crate::game::client::server::{program, respond, server};
use crate::protocol::NetSessionDumpLib;

/// Returns a `metaLib` for the session's remote service library or router `kernel_router.so`.
#[native(method)]
pub fn dump_lib(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(target) = session(vm, this) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = NetSessionDumpLib {
        program: program(vm)?,
        net_session: target.reference(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_net_session_dump_lib(&message),
        |vm, library| {
            let Some(library) = library else {
                return Ok(Value::Null);
            };

            object::create_of(vm, Kind::MetaLib, GameObject::MetaLib, library)
        },
    ))
}
