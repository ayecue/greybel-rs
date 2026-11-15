use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};
use greyhack_behaviour::ROOT_USER;

use super::is_admin;
use crate::error::IntrinsicError;
use crate::game::client::object;
use crate::game::client::object::Access;
use crate::game::client::server::{program, respond, server};
use crate::game::session_mut;
use crate::protocol::AdminSudo;

/// Switches the current session to root on its computer for root-only script operations.
/// Not available in Grey Hack.
#[native(method)]
pub fn sudo(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    if !is_admin(vm, this, "sudo") {
        return Ok(NativeOutput::ready(Value::Null));
    }

    let message = AdminSudo {
        program: program(vm)?,
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_sudo(&message),
        |vm, root| {
            let root = Access::new(object::handle(vm, root)?);
            let session = session_mut(vm)?;

            session.shell = root;
            ROOT_USER.clone_into(&mut session.user);

            Ok::<_, IntrinsicError>(true)
        },
    ))
}
