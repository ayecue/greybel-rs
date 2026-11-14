use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::library;
use crate::error::IntrinsicError;
use crate::game::client::failure::failure_value;
use crate::game::client::object;
use crate::game::client::object::{GameObject, Kind};
use crate::game::client::server::{program, respond, server};
use crate::protocol::{MetaLibDebugTools, ObjectOutcome};

#[native(method)]
pub fn debug_tools(
    vm: &mut Vm,
    this: &Value,
    user: &Value,
    password: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let (Some(library), Some(user), Some(password)) =
        (library(vm, this), user.as_str(), password.as_str())
    else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = MetaLibDebugTools {
        program: program(vm)?,
        library,
        user: user.into(),
        password: password.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_meta_lib_debug_tools(&message),
        |vm, outcome| {
            let value = match outcome {
                ObjectOutcome::Object(tools) => {
                    object::create_of(vm, Kind::DebugLibrary, GameObject::DebugLibrary, tools)?
                }
                ObjectOutcome::Null => Value::Null,
                ObjectOutcome::False => Value::FALSE,
                ObjectOutcome::Failure(failure) => failure_value("debug_tools", &failure)?,
            };

            Ok::<_, IntrinsicError>(value)
        },
    ))
}
