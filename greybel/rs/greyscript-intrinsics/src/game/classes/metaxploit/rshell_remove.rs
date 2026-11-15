use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::metaxploit_or_crash;
use crate::error::IntrinsicError;
use crate::game::client::login_encryption::LoginEncryption;
use crate::game::client::object::{self, GameObject};
use crate::game::client::server::{program, raise, server, settle};
use crate::protocol::{MetaxploitRshellRemove, RshellRemoveSelection};

#[native(method)]
pub fn rshell_remove(
    vm: &mut Vm,
    this: &Value,
    shells: Option<&Value>,
) -> Result<NativeFuture, IntrinsicError> {
    if !object::game_map(vm, this) {
        return Ok(NativeOutput::ready(Value::Null));
    }

    LoginEncryption::deny_in(vm, "rshell_remove")?;

    let Some(metaxploit) = metaxploit_or_crash(vm, this)? else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    // Argument errors travel to the world because the game checks the service before validating them.
    let shells = selection(vm, shells);
    let message = MetaxploitRshellRemove {
        program: program(vm)?,
        metaxploit,
        shells,
    };

    Ok(settle(
        server(vm)?.send_metaxploit_rshell_remove(&message),
        |_, result| {
            result
                .map_err(|failure| raise("rshell_remove", failure))?
                .response("rshell_remove")
        },
    ))
}

fn selection(vm: &Vm, shells: Option<&Value>) -> RshellRemoveSelection {
    let Some(shells) = shells else {
        return RshellRemoveSelection::All;
    };
    let Value::List(list) = shells else {
        return RshellRemoveSelection::InvalidType;
    };
    let mut ids = Vec::new();

    for value in list.items(vm.heap()) {
        match object::any(vm, value) {
            Some(GameObject::Shell(access)) if access.computer.is_some() => {
                ids.push(access.reference());
            }
            _ => return RshellRemoveSelection::InvalidItem,
        }
    }

    RshellRemoveSelection::Shells(ids)
}
