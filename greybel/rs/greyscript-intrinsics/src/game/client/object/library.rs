use greybel_interpreter::{NativeFuture, Value, Vm};

use super::{GameObject, Kind};
use crate::game::client::server::{program, respond, server};
use crate::protocol::{IncludedLibrary, PlayerIncludeLib};

/// Returns the library object resolved by the game server from `path` on the session computer.
pub(in crate::game) fn from_include(
    vm: &Vm,
    path: &str,
) -> Result<NativeFuture, crate::error::IntrinsicError> {
    let message = PlayerIncludeLib {
        program: program(vm)?,
        path: path.to_owned().into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_player_include_lib(&message),
        move |vm, library| {
            let (kind, id, object): (Kind, _, fn(_) -> GameObject) = match library {
                IncludedLibrary::Null => return Ok(Value::Null),
                IncludedLibrary::Metaxploit(id) => (Kind::Metaxploit, id, GameObject::Metaxploit),
                IncludedLibrary::Crypto(id) => (Kind::Crypto, id, GameObject::Crypto),
                IncludedLibrary::AptClient(id) => (Kind::AptClient, id, GameObject::AptClient),
                IncludedLibrary::Blockchain(id) => (Kind::Blockchain, id, GameObject::Blockchain),
                IncludedLibrary::SmartAppliance(id) => {
                    (Kind::SmartAppliance, id, GameObject::SmartAppliance)
                }
                IncludedLibrary::TrafficNet(id) => (Kind::TrafficNet, id, GameObject::TrafficNet),
                IncludedLibrary::Service(id) => (Kind::Service, id, GameObject::Service),
            };

            super::create_of(vm, kind, object, id)
        },
    ))
}
