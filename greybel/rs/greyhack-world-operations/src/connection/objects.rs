//! Connection-local script objects keyed by [`ObjectId`]. Entries retain the world row keys they access
//! until released by the program.

use std::collections::{HashMap, HashSet};

use greyhack_behaviour::ServiceId;
use greyhack_world::{CoinId, ComputerId, Email, NodeId, RouterId, WalletId};

use crate::protocol::{ObjectId, ScriptFailure};
use crate::{OperationError, Result};

/// Computer and user credentials for an object (`HelperValComputer`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Access {
    pub computer: ComputerId,
    pub user: String,
    /// Marks FTP access, which exposes fewer methods.
    pub ftp: bool,
    /// Public address of the computer the player came from, which the `remove` and `bounce` entries of the system
    /// log name (`HelperValComputer.prevBounceIP`).
    pub prev_ip: String,
    /// Number of connections between the player's computer and this one (`HelperValComputer.numBounces`).
    pub bounces: u32,
}

/// File `node` accessed as `access`, with its last observed `path` (`HelperValFile.pathFile`). Moves and
/// renames through this object update the node and path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileRef {
    pub access: Access,
    pub node: NodeId,
    pub path: String,
}

/// Library at `path` with access identity `access`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryRef {
    pub access: Access,
    pub path: String,
    /// Computer whose metaxploit opened the library (`HelperValMetaLib.metaxploitComputerID`).
    pub origin: ComputerId,
}

/// `metaLib` target backed by a library file or router `kernel_router.so`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetaLibRef {
    File(LibraryRef),
    Kernel(KernelRef),
}

/// Router `kernel_router.so` reached by `net_use` at `address` from `origin`
/// See `HelperValMetaLib.routerAddress` and `metaxploitComputerID`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelRef {
    pub router: RouterId,
    pub address: String,
    pub origin: ComputerId,
    /// Bounces of the metaxploit that opened the session (`HelperValMetaLib.numBounces`).
    pub bounces: u32,
}

/// `metaxploit` access identity and library version used for scans and exploits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetaxploitRef {
    pub access: Access,
    pub version: String,
}

/// Service `service` reached through computer access `access`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceRef {
    pub access: Access,
    pub service: ServiceId,
}

/// `net_use` target: a service library opened from `origin`, or a router.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetSessionRef {
    Service { file: FileRef, origin: ComputerId },
    Router(KernelRef),
}

impl NetSessionRef {
    /// Returns the metaxploit computer that opened the session (`HelperValFile.prevBounceIP` stores its public address).
    #[must_use]
    pub const fn origin(&self) -> ComputerId {
        match self {
            Self::Service { origin, .. } | Self::Router(KernelRef { origin, .. }) => *origin,
        }
    }
}

/// Logged-in mail account accessed from `access`'s computer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MailRef {
    pub access: Access,
    pub address: String,
    /// Inbox snapshot from login; sending mail does not update it (`HelperMailUser.userMail`).
    pub emails: Vec<Email>,
}

/// Coin `coin` accessed through the blockchain library on `computer`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoinRef {
    pub coin: CoinId,
    pub computer: ComputerId,
}

/// Sub-wallet keyed by coin `coin` and user `user`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubWalletRef {
    pub coin: CoinId,
    pub user: String,
}

/// CTF event `event` obtained on `computer`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CtfRef {
    pub computer: ComputerId,
    pub event: String,
}

/// Script object variants grouped by intrinsic class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Object {
    /// Shell access that also acts as the computer object returned by `host_computer`.
    Shell(Access),
    /// Reverse-shell client shell from `rshell_server`, with one object per process (`Shell.reversePID`).
    ReverseShell(Access, u32),
    Computer(Access),
    File(FileRef),
    Router(RouterId),
    Metaxploit(MetaxploitRef),
    Crypto(Access),
    AptClient(Access),
    Blockchain(Access),
    SmartAppliance(Access),
    TrafficNet(Access),
    Service(ServiceRef),
    MetaLib(MetaLibRef),
    NetSession(NetSessionRef),
    /// Debug tools for a library opened with a debug account.
    DebugLibrary(LibraryRef),
    MetaMail(MailRef),
    /// Authenticated wallet.
    Wallet(WalletId),
    /// Coin authenticated with creator credentials.
    Coin(CoinRef),
    SubWallet(SubWalletRef),
    Ctf(CtfRef),
}

impl Object {
    /// Checks whether `other` has the same game identity using `HelperScript`'s `Add*` comparisons. Mail
    /// objects match by address, coins by coin, CTF objects by event, and other objects by all referenced
    /// data. A shell and computer with the same access share one object, so `host_computer` and
    /// `AddComputer` return that object.
    fn is_same_as(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Shell(shell), Self::Computer(computer))
            | (Self::Computer(computer), Self::Shell(shell)) => shell == computer,
            (Self::MetaMail(mail), Self::MetaMail(other)) => {
                mail.access == other.access && mail.address == other.address
            }
            (Self::Coin(coin), Self::Coin(other)) => coin.coin == other.coin,
            (Self::Ctf(ctf), Self::Ctf(other)) => ctf.event == other.event,
            _ => self == other,
        }
    }

    /// Returns the computer or router referenced by this object, if any.
    fn reach(&self) -> Option<Reach> {
        let computer = |access: &Access| Some(Reach::Computer(access.computer));

        match self {
            Self::Shell(access)
            | Self::ReverseShell(access, _)
            | Self::Computer(access)
            | Self::Crypto(access)
            | Self::AptClient(access)
            | Self::Blockchain(access)
            | Self::SmartAppliance(access)
            | Self::TrafficNet(access) => computer(access),
            Self::File(file) | Self::NetSession(NetSessionRef::Service { file, .. }) => {
                computer(&file.access)
            }
            Self::Metaxploit(metaxploit) => computer(&metaxploit.access),
            Self::Service(service) => computer(&service.access),
            Self::MetaLib(MetaLibRef::File(library)) | Self::DebugLibrary(library) => {
                computer(&library.access)
            }
            Self::MetaMail(mail) => computer(&mail.access),
            Self::Ctf(ctf) => Some(Reach::Computer(ctf.computer)),
            Self::Router(router)
            | Self::MetaLib(MetaLibRef::Kernel(KernelRef { router, .. }))
            | Self::NetSession(NetSessionRef::Router(KernelRef { router, .. })) => {
                Some(Reach::Router(*router))
            }
            Self::Wallet(_) | Self::Coin(_) | Self::SubWallet(_) => None,
        }
    }
}

/// Computer or router referenced by an object.
#[derive(Debug, Clone, Copy)]
enum Reach {
    Computer(ComputerId),
    Router(RouterId),
}

/// Objects owned by one connection, keyed by ID.
#[derive(Debug, Default)]
pub struct ObjectTable {
    objects: HashMap<u32, Object>,
    /// Objects interrupted because their computer rebooted after registration (`CheckBreakConnecion`).
    interruptions: HashSet<u32>,
    next: u32,
}

/// Defines typed accessors that reject unknown IDs and incorrect object classes.
macro_rules! accessors {
    ($($(#[$doc:meta])* $name:ident => $type:ty, $($variant:ident)|+;)*) => {
        $(
            $(#[$doc])*
            ///
            /// # Errors
            ///
            /// Returns [`OperationError::UnknownObject`] for a missing ID or wrong class, and
            /// [`ScriptFailure::ConnectionInterrupted`] if the computer rebooted after registration.
            pub fn $name(&self, id: ObjectId) -> Result<&$type> {
                self.check_connection(id)?;

                match self.objects.get(&id.id) {
                    $(Some(Object::$variant(object)))|+ => Ok(object),
                    _ => Err(OperationError::UnknownObject),
                }
            }
        )*
    };
}

impl ObjectTable {
    accessors! {
        file => FileRef, File;
        router => RouterId, Router;
        metaxploit => MetaxploitRef, Metaxploit;
        crypto => Access, Crypto;
        apt_client => Access, AptClient;
        blockchain => Access, Blockchain;
        smart_appliance => Access, SmartAppliance;
        traffic_net => Access, TrafficNet;
        service => ServiceRef, Service;
        meta_lib => MetaLibRef, MetaLib;
        net_session => NetSessionRef, NetSession;
        debug_library => LibraryRef, DebugLibrary;
        meta_mail => MailRef, MetaMail;
        wallet => WalletId, Wallet;
        coin => CoinRef, Coin;
        sub_wallet => SubWalletRef, SubWallet;
        ctf => CtfRef, Ctf;
    }

    /// Returns shell access for `id`, including reverse shells.
    ///
    /// # Errors
    ///
    /// Returns [`OperationError::UnknownObject`] for a missing ID or wrong class, and
    /// [`ScriptFailure::ConnectionInterrupted`] if the computer rebooted after registration.
    pub fn shell(&self, id: ObjectId) -> Result<&Access> {
        self.check_connection(id)?;

        match self.objects.get(&id.id) {
            Some(Object::Shell(access) | Object::ReverseShell(access, _)) => Ok(access),
            _ => Err(OperationError::UnknownObject),
        }
    }

    /// Returns computer access from either a computer object or a host shell.
    ///
    /// # Errors
    ///
    /// Returns [`OperationError::UnknownObject`] for a missing ID or wrong class, and
    /// [`ScriptFailure::ConnectionInterrupted`] if the computer rebooted after registration.
    pub fn computer(&self, id: ObjectId) -> Result<&Access> {
        self.check_connection(id)?;

        match self.objects.get(&id.id) {
            Some(
                Object::Shell(access) | Object::ReverseShell(access, _) | Object::Computer(access),
            ) => Ok(access),
            _ => Err(OperationError::UnknownObject),
        }
    }

    /// Returns the computer and user access for a script session.
    ///
    /// # Errors
    ///
    /// Returns [`OperationError::UnknownObject`] for a missing ID or wrong class, and
    /// [`ScriptFailure::ConnectionInterrupted`] if the computer rebooted after registration.
    pub fn session(&self, id: ObjectId) -> Result<&Access> {
        self.shell(id)
    }

    /// Adds `object` and returns its local ID. If [`Object::is_same_as`] matches an existing uninterrupted
    /// entry, returns that entry's ID so scripts preserve game object identity.
    pub fn add(&mut self, object: Object) -> ObjectId {
        if let Some((&id, _)) = self
            .objects
            .iter()
            .filter(|(id, _)| !self.interruptions.contains(id))
            .find(|(_, current)| current.is_same_as(&object))
        {
            // A shell matching a computer's access replaces the entry and continues to serve both roles.
            if matches!(object, Object::Shell(_)) {
                self.objects.insert(id, object);
            }

            return ObjectId { id };
        }

        let id = self.next;

        self.next = self.next.wrapping_add(1);
        self.objects.insert(id, object);

        ObjectId { id }
    }

    /// Removes released `ids` and their interruption state.
    pub fn release(&mut self, ids: &[ObjectId]) {
        for id in ids {
            self.objects.remove(&id.id);
            self.interruptions.remove(&id.id);
        }
    }

    /// Marks objects reached through a rebooted `computer` as interrupted. Later operations raise
    /// [`ScriptFailure::ConnectionInterrupted`] like `CheckBreakConnecion`. Router objects are interrupted
    /// when the router's computer reboots.
    pub fn interrupt(&mut self, computer: ComputerId) {
        let objects = self
            .objects
            .iter()
            .filter(|(_, object)| match object.reach() {
                Some(Reach::Computer(id)) => id == computer,
                Some(Reach::Router(router)) => router.computer() == computer,
                None => false,
            })
            .map(|(id, _)| *id);

        self.interruptions.extend(objects);
    }

    /// Returns [`ScriptFailure::ConnectionInterrupted`] when object `id` was interrupted.
    fn check_connection(&self, id: ObjectId) -> Result<()> {
        if self.interruptions.contains(&id.id) {
            return Err(OperationError::Script(ScriptFailure::ConnectionInterrupted));
        }

        Ok(())
    }

    /// Returns mutable mail reference `id` so deletion updates its stored inbox.
    ///
    /// # Errors
    ///
    /// Returns [`OperationError::UnknownObject`] for a missing mail object, or
    /// [`ScriptFailure::ConnectionInterrupted`] if its computer rebooted after registration.
    pub fn meta_mail_mut(&mut self, id: ObjectId) -> Result<&mut MailRef> {
        self.check_connection(id)?;

        match self.objects.get_mut(&id.id) {
            Some(Object::MetaMail(mail)) => Ok(mail),
            _ => Err(OperationError::UnknownObject),
        }
    }

    /// Counts objects owned by this connection.
    #[must_use]
    pub fn len(&self) -> usize {
        self.objects.len()
    }

    /// Returns mutable file reference `id` for path updates after moves and renames.
    ///
    /// # Errors
    ///
    /// Returns [`OperationError::UnknownObject`] for a missing file object, or
    /// [`ScriptFailure::ConnectionInterrupted`] if its computer rebooted after registration.
    pub fn file_mut(&mut self, id: ObjectId) -> Result<&mut FileRef> {
        self.check_connection(id)?;

        match self.objects.get_mut(&id.id) {
            Some(Object::File(file)) => Ok(file),
            _ => Err(OperationError::UnknownObject),
        }
    }
}
