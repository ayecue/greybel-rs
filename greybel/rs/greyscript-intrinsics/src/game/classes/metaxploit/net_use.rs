use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use crate::error::IntrinsicError;
use crate::game::client::login_encryption::LoginEncryption;
use crate::game::client::object;
use crate::game::client::object::{GameObject, Kind};
use crate::game::client::receiver::{self, receiver};
use crate::game::client::server::{program, raise, server, settle};
use crate::protocol::{MetaxploitNetUse, NetUseTarget};

/// Opens a `netSession` to the service at `ipAddress:port` for remote-library access through `dump_lib`.
#[native(method)]
pub fn net_use(
    vm: &mut Vm,
    this: &Value,
    #[name = "ipAddress"] address: &Value,
    #[default(0)] port: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    if !object::game_map(vm, this) {
        return Ok(NativeOutput::ready(Value::Null));
    }

    let (Some(address), Some(port)) = (address.as_str(), port.as_int()) else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    LoginEncryption::deny_in(vm, "net_use")?;

    let metaxploit =
        receiver::<receiver::Metaxploit>(vm, this).map(|metaxploit| metaxploit.reference());

    let message = MetaxploitNetUse {
        program: program(vm)?,
        metaxploit,
        address: address.into(),
        port: u16::try_from(port).unwrap_or(0),
    };

    Ok(settle(
        server(vm)?.send_metaxploit_net_use(&message),
        |vm, result| {
            let target = result.map_err(|failure| raise("net_use", failure))?;
            let value = match target {
                NetUseTarget::Null => Value::Null,
                NetUseTarget::Session(session) => {
                    object::create_of(vm, Kind::NetSession, GameObject::NetSession, session)?
                }
            };

            Ok::<_, IntrinsicError>(value)
        },
    ))
}
