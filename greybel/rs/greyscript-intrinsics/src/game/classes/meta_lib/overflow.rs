use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use crate::error::IntrinsicError;
use crate::game::client::object::{Access, FileObject};
use crate::game::client::receiver::receiver;
use crate::game::client::server::{program, raise, server, settle};
use crate::game::client::{object, receiver};
use crate::protocol::{ExploitObject, MetaLibOverflow};

/// Exploits the weakness at `memAddress` using `unsecValue`.
#[native(method)]
pub fn overflow(
    vm: &mut Vm,
    this: &Value,
    #[name = "memAddress"] address: &Value,
    #[name = "unsecValue"] unsec_value: &Value,
    #[default("")] args: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    if !object::game_map(vm, this) {
        return Err(IntrinsicError::InvalidArguments("overflow"));
    }

    let (Some(address), Some(unsec_value)) = (address.as_str(), unsec_value.as_str()) else {
        return Err(IntrinsicError::InvalidArguments("overflow"));
    };
    let Some(source) = receiver::<receiver::MetaLib>(vm, this) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = MetaLibOverflow {
        program: program(vm)?,
        library: source.reference(),
        address: address.into(),
        unsec_value: unsec_value.into(),
        args: args.as_str().map(Into::into),
    };

    Ok(settle(
        server(vm)?.send_meta_lib_overflow(&message),
        |vm, exploit| {
            let exploit = exploit.map_err(|failure| raise("overflow", failure))?;

            let value = match exploit.object {
                ExploitObject::Null => Value::Null,
                ExploitObject::DisabledNetwork => Value::from(
                    "Exploit failed: the remote server has been temporarily disabled due to non-payment",
                ),
                ExploitObject::True => Value::TRUE,
                ExploitObject::False => Value::FALSE,
                ExploitObject::Shell(shell) => {
                    let shell = Access::new(object::handle(vm, shell)?);

                    object::create_shell(vm, shell)
                }
                ExploitObject::Computer(computer) => {
                    let computer = Access::new(object::handle(vm, computer)?);

                    object::create_computer(vm, computer)
                }
                ExploitObject::File(file) => {
                    let file = FileObject {
                        object: object::handle(vm, file)?,
                        ftp: false,
                    };

                    object::create_file(vm, file)
                }
            };

            Ok::<_, IntrinsicError>(value)
        },
    ))
}
