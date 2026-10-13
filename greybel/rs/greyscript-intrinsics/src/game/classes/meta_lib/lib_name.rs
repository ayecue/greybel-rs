use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use crate::error::IntrinsicError;
use crate::game::client::receiver;
use crate::game::client::receiver::receiver;
use crate::game::client::server::{program, respond, server};
use crate::protocol::MetaLibLibName;

#[native(method)]
pub fn lib_name(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(source) = receiver::<receiver::MetaLib>(vm, this) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = MetaLibLibName {
        program: program(vm)?,
        library: source.reference(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_meta_lib_lib_name(&message),
        |_, name| name,
    ))
}
