use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::library;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::MetaLibIsPatched;

#[native(method)]
pub fn is_patched(
    vm: &mut Vm,
    this: &Value,
    #[default(0)] getdate: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let Some(library) = library(vm, this) else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    if getdate.as_number().is_none() {
        return Ok(NativeOutput::ready(Value::Null));
    }

    let message = MetaLibIsPatched {
        program: program(vm)?,
        library,
        date: getdate.as_number().is_some_and(|value| value != 0.0),
    };

    Ok(respond(
        vm,
        server(vm)?.send_meta_lib_is_patched(&message),
        |_, state| state.response(),
    ))
}
