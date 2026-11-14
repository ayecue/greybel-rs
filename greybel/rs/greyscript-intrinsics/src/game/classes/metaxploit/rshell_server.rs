use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::metaxploit_or_crash;
use crate::error::IntrinsicError;
use crate::game::client::failure::failure_value;
use crate::game::client::login_encryption::LoginEncryption;
use crate::game::client::object;
use crate::game::client::object::Access;
use crate::game::client::server::{program, raise, server, settle};
use crate::protocol::{MetaxploitRshellServer, RshellServerResponse};

#[native(method)]
pub fn rshell_server(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    if !object::game_map(vm, this) {
        return Ok(NativeOutput::ready(Value::Null));
    }

    LoginEncryption::deny_in(vm, "rshell_server")?;

    let Some(metaxploit) = metaxploit_or_crash(vm, this)? else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    let message = MetaxploitRshellServer {
        program: program(vm)?,
        metaxploit,
    };

    Ok(settle(
        server(vm)?.send_metaxploit_rshell_server(&message),
        |vm, result| {
            let response = result.map_err(|failure| raise("rshell_server", failure))?;
            let value = match response {
                RshellServerResponse::Null => Value::Null,
                RshellServerResponse::Failure(failure) => failure_value("rshell_server", &failure)?,
                RshellServerResponse::Clients(clients) => {
                    let mut shells = Vec::with_capacity(clients.len());

                    for client in clients {
                        let shell = Access {
                            computer: Some(object::handle(vm, client.computer)?),
                            ..Access::new(object::handle(vm, client.shell)?)
                        };

                        shells.push(object::create_shell(vm, shell));
                    }

                    Value::List(vm.heap_mut().add_list(shells))
                }
            };

            Ok::<_, IntrinsicError>(value)
        },
    ))
}
