//! Script-visible game objects: sealed maps with `__isa` method type maps and `classID`, backed by [`GameObject`].

mod create;
mod kind;
mod library;
mod payload;
mod prototype;

pub(crate) use create::{
    create, create_computer, create_file, create_of, create_partial, create_shell, handle,
};
use greybel_interpreter::{Value, Vm};
use greybel_symbols::names::ISA;
pub(crate) use kind::Kind;
pub(in crate::game) use library::from_include;
pub(crate) use payload::{Access, ComputerAddress, FileObject};

use crate::protocol::{ObjectId, Port};
use crate::runtime::Handle;

/// Identity backing a game object.
/// Most variants wrap server objects; ports store values, while `get_greybel_util()` objects identify sandbox state.
#[derive(Debug)]
pub(crate) enum GameObject {
    Shell(Access),
    Computer(Access),
    File(FileObject),
    Port(Port),
    Router(Handle),
    Metaxploit(Handle),
    Crypto(Handle),
    AptClient(Handle),
    Blockchain(Handle),
    /// Authenticated wallet.
    Wallet(Handle),
    /// Coin accessed with creator credentials.
    Coin(Handle),
    SubWallet(Handle),
    MetaMail(Handle),
    Service(Handle),
    MetaLib(Handle),
    NetSession(Handle),
    Ctf(Handle),
    SmartAppliance(Handle),
    TrafficNet(Handle),
    DebugLibrary(Handle),
    /// Sandbox-only `get_greybel_util()`, absent from Grey Hack.
    Admin,
    /// Network at a public address accessed through `get_greybel_util()`.
    AdminNetwork(String),
    /// Computer accessed through `get_greybel_util()`.
    AdminComputer(ComputerAddress),
}

/// Defines identity within each object kind: server-object identity, port values (`Puerto.Equals`), or `get_greybel_util()` target identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum ObjectKey {
    Server(u32),
    Port {
        number: u16,
        internal_port: u16,
        service: String,
        lan_ip: String,
        closed: bool,
        visible: bool,
    },
    Admin,
    AdminNetwork(String),
    AdminComputer {
        public_ip: String,
        lan_ip: String,
    },
}

impl GameObject {
    /// Returns the identity key used to compare objects of this kind.
    pub(crate) fn key(&self) -> ObjectKey {
        match (self.reference(), self) {
            (Some(id), _) => ObjectKey::Server(id.id),
            (None, Self::Port(port)) => ObjectKey::Port {
                number: port.number,
                internal_port: port.internal_port,
                service: port.service.clone(),
                lan_ip: port.lan_ip.clone(),
                closed: port.closed,
                visible: port.visible,
            },
            (None, Self::AdminNetwork(public_ip)) => ObjectKey::AdminNetwork(public_ip.clone()),
            (None, Self::AdminComputer(address)) => ObjectKey::AdminComputer {
                public_ip: address.public_ip.clone(),
                lan_ip: address.lan_ip.clone(),
            },
            (None, _) => ObjectKey::Admin,
        }
    }

    /// Returns the backing server object, if this kind has one.
    pub(crate) fn reference(&self) -> Option<ObjectId> {
        match self {
            Self::Shell(access) | Self::Computer(access) => Some(access.reference()),
            Self::File(file) => Some(file.reference()),
            Self::Router(handle)
            | Self::Metaxploit(handle)
            | Self::Crypto(handle)
            | Self::AptClient(handle)
            | Self::Blockchain(handle)
            | Self::Wallet(handle)
            | Self::Coin(handle)
            | Self::SubWallet(handle)
            | Self::MetaMail(handle)
            | Self::Service(handle)
            | Self::MetaLib(handle)
            | Self::NetSession(handle)
            | Self::Ctf(handle)
            | Self::SmartAppliance(handle)
            | Self::TrafficNet(handle)
            | Self::DebugLibrary(handle) => Some(handle.reference()),
            Self::Port(_) | Self::Admin | Self::AdminNetwork(_) | Self::AdminComputer(_) => None,
        }
    }
}

/// Tests whether `this` is a game-state object map or class type map.
#[must_use]
pub(crate) fn game_map(vm: &Vm, this: &Value) -> bool {
    let Value::Map(id) = this else {
        return false;
    };

    id.handle::<GameObject>(vm.heap()).is_some() || prototype::is_type_map(vm, *id)
}

/// Extracts a sealed game-object map from `this` without checking a method.
#[must_use]
pub(crate) fn any<'v>(vm: &'v Vm, this: &Value) -> Option<&'v GameObject> {
    let Value::Map(id) = this else {
        return None;
    };

    id.handle::<GameObject>(vm.heap())
}

/// Extracts a game object from `this` if its type provides `method`.
#[must_use]
pub(crate) fn this<'v>(vm: &'v Vm, this: &Value, method: &str) -> Option<&'v GameObject> {
    let Value::Map(id) = this else {
        return None;
    };
    let heap = vm.heap();
    let object = id.handle::<GameObject>(heap)?;
    let Some(Value::Map(isa)) = id.get_str(heap, ISA) else {
        return None;
    };

    isa.get_str(heap, method)?;

    Some(object)
}
