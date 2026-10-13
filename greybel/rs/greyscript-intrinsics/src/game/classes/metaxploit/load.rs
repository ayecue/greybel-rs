use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::metaxploit;
use crate::error::IntrinsicError;
use crate::game::client::object;
use crate::game::client::object::{GameObject, Kind};
use crate::game::client::server::{program, respond, server};
use crate::protocol::MetaxploitLoad;

/// Creates a `metaLib` by loading `path` from this computer.
#[native(method)]
pub fn load(vm: &mut Vm, this: &Value, path: &Value) -> Result<NativeFuture, IntrinsicError> {
    if !object::game_map(vm, this) {
        return Ok(NativeOutput::ready(Value::Null));
    }

    let Some(path) = path.as_str() else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    if path.is_empty() {
        return Err(IntrinsicError::InvalidArguments("load"));
    }

    let Some(metaxploit) = metaxploit(vm, this) else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    let message = MetaxploitLoad {
        program: program(vm)?,
        metaxploit,
        path: path.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_metaxploit_load(&message),
        |vm, library| {
            let Some(library) = library else {
                return Ok(Value::Null);
            };

            object::create_of(vm, Kind::MetaLib, GameObject::MetaLib, library)
        },
    ))
}
