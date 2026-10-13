use greybel_interpreter::{NativeFuture, NativeOutput, PrintOptions, Value, Vm, native};

use crate::error::IntrinsicError;
use crate::game::client::failure::lan_text;
use crate::game::client::login_encryption::LoginEncryption;
use crate::game::client::object;
use crate::game::client::object::{GameObject, Kind};
use crate::game::client::receiver::{self, receiver};
use crate::game::client::server::{program, raise, server, settle};
use crate::protocol::{MetaxploitNetUse, NetUseFailure, NetUseTarget};

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

    let address = address.to_owned();
    let message = MetaxploitNetUse {
        program: program(vm)?,
        metaxploit,
        address: address.clone().into(),
        port: u16::try_from(port).unwrap_or(0),
    };

    Ok(settle(
        server(vm)?.send_metaxploit_net_use(&message),
        move |vm, result| {
            let target = result.map_err(|failure| raise("net_use", failure))?;
            let value = match target {
                NetUseTarget::Null => Value::Null,
                NetUseTarget::Session(session) => {
                    object::create_of(vm, Kind::NetSession, GameObject::NetSession, session)?
                }
                NetUseTarget::Failure(failure) => {
                    // `net_use` prints the session-open failure and returns null.
                    vm.stdout()
                        .print(&net_use_text(&failure, &address), PrintOptions::default());

                    Value::Null
                }
            };

            Ok::<_, IntrinsicError>(value)
        },
    ))
}

/// Formats `MetaExploitIntrinsics.net_use` failure text for `address`.
fn net_use_text(failure: &NetUseFailure, address: &str) -> String {
    match failure {
        NetUseFailure::InvalidAddress => format!("Invalid ip address: {address}"),
        NetUseFailure::UnknownAddress => "Ip address not found.".to_owned(),
        NetUseFailure::MainRouterDown => {
            "net_use: the main router of the network is down.".to_owned()
        }
        NetUseFailure::Lan(failure) => lan_text(*failure).to_owned(),
        NetUseFailure::UnknownComputer => "Error: LAN computer not found.".to_owned(),
        NetUseFailure::MissingPort => "Port not found.".to_owned(),
        NetUseFailure::ClosedPort => "can't connect: port closed.".to_owned(),
        NetUseFailure::InvalidLibrary(library) => format!("Error: Not a valid {library} library."),
        NetUseFailure::UnexpectedLibrary(name) => {
            format!("Unexpected library found. Not a valid {name} library.")
        }
    }
}
