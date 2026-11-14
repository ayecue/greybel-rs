use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::reference;
use crate::error::IntrinsicError;
use crate::game::client::failure::failure_value;
use crate::game::client::object;
use crate::game::client::object::{Access, FileObject, GameObject, Kind};
use crate::game::client::server::{program, respond, server};
use crate::protocol::{DebugLibraryPayload, PartialObject, ZeroDayPayload};

#[native(method)]
pub fn payload(
    vm: &mut Vm,
    this: &Value,
    #[name = "memZone"] mem_zone: &Value,
    #[name = "pathFile"]
    #[default("")]
    path_file: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    if !object::game_map(vm, this) {
        return Ok(NativeOutput::ready(Value::Null));
    }

    let (Some(mem_zone), Some(path_file)) = (mem_zone.as_str(), path_file.as_str()) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let Some(reference) = reference(vm, this) else {
        return Ok(NativeOutput::ready("error: file not found"));
    };
    let message = DebugLibraryPayload {
        program: program(vm)?,
        library: reference.reference(),
        mem_zone: mem_zone.into(),
        path: path_file.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_debug_library_payload(&message),
        |vm, result| {
            let partial_objects = match result {
                ZeroDayPayload::Null => return Ok(Value::Null),
                ZeroDayPayload::Failure(failure) => {
                    return failure_value("payload", &failure);
                }
                ZeroDayPayload::Objects(partial_objects) => partial_objects,
            };
            let mut objects = Vec::with_capacity(partial_objects.len());

            for partial in partial_objects {
                let (kind, object, methods) = match partial {
                    PartialObject::Computer { computer, methods } => (
                        Kind::Computer,
                        GameObject::Computer(Access::new(object::handle(vm, computer)?)),
                        methods,
                    ),
                    PartialObject::File { file, methods } => (
                        Kind::File,
                        GameObject::File(FileObject {
                            object: object::handle(vm, file)?,
                            ftp: false,
                        }),
                        methods,
                    ),
                    PartialObject::MetaLib { library, methods } => (
                        Kind::MetaLib,
                        GameObject::MetaLib(object::handle(vm, library)?),
                        methods,
                    ),
                };

                objects.push(object::create_partial(vm, kind, object, &methods));
            }

            Ok::<_, IntrinsicError>(Value::List(vm.heap_mut().add_list(objects)))
        },
    ))
}
