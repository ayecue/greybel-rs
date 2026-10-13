use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use crate::error::IntrinsicError;
use crate::game::client::receiver;
use crate::game::client::receiver::receiver;
use crate::game::client::server::{program, respond, server};
use crate::protocol::MetaLibVersion;

#[native(method)]
pub fn version(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(source) = receiver::<receiver::MetaLib>(vm, this) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = MetaLibVersion {
        program: program(vm)?,
        library: source.reference(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_meta_lib_version(&message),
        |_, version| version,
    ))
}
