use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::file;
use crate::error::IntrinsicError;
use crate::game::client::object;
use crate::game::client::server::{program, respond, server};
use crate::protocol::FileParent;

#[native(method)]
pub fn parent(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(file) = file(vm, this, "parent") else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let file = file.clone();
    let message = FileParent {
        program: program(vm)?,
        file: file.reference(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_file_parent(&message),
        move |vm, parent| {
            let Some(parent) = parent else {
                return Ok(Value::Null);
            };
            let parent = file.with_object(object::handle(vm, parent)?);

            Ok::<_, IntrinsicError>(object::create_file(vm, parent))
        },
    ))
}
