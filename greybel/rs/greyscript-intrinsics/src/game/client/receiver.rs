//! Extracts game objects from native `self` values, with optional method checks.

use greybel_interpreter::{Value, Vm};

use super::object::{self, Access, GameObject};
use crate::error::IntrinsicError;
use crate::runtime::Handle;

/// Extracts one [`GameObject`] payload variant from native `self`.
/// Implemented by marker types declared in `receivers!`.
pub(in crate::game) trait Receiver {
    type Data;

    fn data(object: &GameObject) -> Option<&Self::Data>;
}

/// Returns the game object behind `this` without checking a method name.
pub(in crate::game) fn receiver<'v, R: Receiver>(vm: &'v Vm, this: &Value) -> Option<&'v R::Data> {
    R::data(object::any(vm, this)?)
}

/// Extracts the requested object from a game map.
/// Wrong-class objects and type maps crash where Grey Hack dereferences a missing helper; non-game maps and non-maps return null.
pub(in crate::game) fn receiver_or_crash<'v, R: Receiver>(
    vm: &'v Vm,
    this: &Value,
) -> Result<Option<&'v R::Data>, IntrinsicError> {
    if !object::game_map(vm, this) {
        return Ok(None);
    }

    match object::any(vm, this).and_then(R::data) {
        Some(data) => Ok(Some(data)),
        None => Err(IntrinsicError::GameCrash),
    }
}

/// Returns the game object behind `this` only if its type provides `method`.
pub(in crate::game) fn receiver_for<'v, R: Receiver>(
    vm: &'v Vm,
    this: &Value,
    method: &str,
) -> Option<&'v R::Data> {
    R::data(object::this(vm, this, method)?)
}

/// Declares a marker type and payload type for each [`GameObject`] variant.
macro_rules! receivers {
    ($($marker:ident => $data:ty, $pattern:pat => $payload:expr;)*) => {
        $(
            pub(in crate::game) struct $marker;

            impl Receiver for $marker {
                type Data = $data;
                fn data(object: &GameObject) -> Option<&Self::Data> {
                    match object {
                        $pattern => Some($payload),
                        _ => None,
                    }
                }
            }
        )*
    };
}

receivers! {
    Shell         => Access,           GameObject::Shell(access) => access;
    Computer      => Access,           GameObject::Computer(access) => access;
    Metaxploit    => Handle,           GameObject::Metaxploit(metaxploit) => metaxploit;
    Crypto        => Handle,           GameObject::Crypto(crypto) => crypto;
    AptClient     => Handle,           GameObject::AptClient(apt_client) => apt_client;
    Blockchain    => Handle,           GameObject::Blockchain(blockchain) => blockchain;
    File          => object::FileObject, GameObject::File(file) => file;
    Port          => crate::protocol::Port, GameObject::Port(port) => port;
    Router        => Handle,           GameObject::Router(router) => router;
    Wallet        => Handle,           GameObject::Wallet(wallet) => wallet;
    Coin          => Handle,           GameObject::Coin(coin) => coin;
    SubWallet     => Handle,           GameObject::SubWallet(sub) => sub;
    MetaMail      => Handle,           GameObject::MetaMail(mail) => mail;
    Service       => Handle,           GameObject::Service(service) => service;
    MetaLib       => Handle,           GameObject::MetaLib(lib) => lib;
    NetSession    => Handle,           GameObject::NetSession(session) => session;
    Ctf           => Handle,           GameObject::Ctf(ctf) => ctf;
    DebugLibrary  => Handle,           GameObject::DebugLibrary(library) => library;
    SmartAppliance => Handle,          GameObject::SmartAppliance(appliance) => appliance;
    TrafficNet    => Handle,           GameObject::TrafficNet(traffic) => traffic;
    AdminNetwork  => String,           GameObject::AdminNetwork(public_ip) => public_ip;
    AdminComputer => object::ComputerAddress, GameObject::AdminComputer(address) => address;
}
