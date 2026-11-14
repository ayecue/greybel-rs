//! Script object handles and their game identity rules. Storage and typed lookup live in the router.

pub use greyhack_world_core::{
    Access, AccessHandle, AccessKind, FileHandle, FileRef, ReverseShellHandle, RouterHandle,
};
pub use greyhack_world_economy::{
    BlockchainHandle, CoinHandle, CoinRef, SubWalletHandle, SubWalletRef, WalletHandle,
};
pub use greyhack_world_intrusion::CryptoHandle;
pub use greyhack_world_libraries::{
    DebugLibraryHandle, KernelRef, LibraryRef, MetaLibHandle, MetaLibRef, MetaxploitHandle,
    MetaxploitRef, NetSessionHandle, NetSessionRef,
};
pub use greyhack_world_npcs::{MailRef, MetaMailHandle};
pub use greyhack_world_player::{CtfHandle, CtfRef};
use greyhack_world_router::protocol::ObjectId;
use greyhack_world_router::{ObjectTable as TreeObjects, ObjectTableHandle};
pub use greyhack_world_services::{AptClientHandle, ServiceHandle, ServiceRef};
pub use greyhack_world_smart_devices::{SmartApplianceHandle, TrafficNetHandle};

use crate::routes::Result;
use crate::{ComputerId, RouterId, WalletId, WorldError};

/// Script object variants used to register typed handles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Object {
    Shell(Access),
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
    DebugLibrary(LibraryRef),
    MetaMail(MailRef),
    Wallet(WalletId),
    Coin(CoinRef),
    SubWallet(SubWalletRef),
    Ctf(CtfRef),
}

/// Typed object lookup and game-specific shared accessors on a tree's object table.
pub trait ObjectTableExt:
    crate::CoreObjectTableExt
    + greyhack_world_smart_devices::ObjectTableExt
    + greyhack_world_libraries::ObjectTableExt
    + greyhack_world_player::ObjectTableExt
{
    /// Registers `object`, reusing an existing game identity where `HelperScript` does.
    fn add(&mut self, object: Object) -> ObjectId;
    /// Returns package-manager access.
    fn apt_client(&self, id: ObjectId) -> Result<Access>;
    /// Returns blockchain access.
    fn blockchain(&self, id: ObjectId) -> Result<Access>;
    /// Returns service access.
    fn service(&self, id: ObjectId) -> Result<ServiceRef>;
    /// Returns wallet identity.
    fn wallet(&self, id: ObjectId) -> Result<WalletId>;
    /// Returns coin access.
    fn coin(&self, id: ObjectId) -> Result<CoinRef>;
    /// Returns sub-wallet access.
    fn sub_wallet(&self, id: ObjectId) -> Result<SubWalletRef>;
}

macro_rules! accessors {
    ($($name:ident => $handle:ident.$field:tt as $result:ty;)*) => {
        $(
            fn $name(&self, id: ObjectId) -> Result<$result> {
                Ok(self.get::<$handle, WorldError>(id)?.$field.clone())
            }
        )*
    };
}

impl ObjectTableExt for TreeObjects<ComputerId> {
    accessors! {
        apt_client => AptClientHandle.0 as Access;
        blockchain => BlockchainHandle.0 as Access;
        service => ServiceHandle.0 as ServiceRef;
        wallet => WalletHandle.0 as WalletId;
        coin => CoinHandle.0 as CoinRef;
        sub_wallet => SubWalletHandle.0 as SubWalletRef;
    }

    fn add(&mut self, object: Object) -> ObjectId {
        match object {
            Object::Shell(access) => self.insert(AccessHandle {
                access,
                kind: AccessKind::Shell,
            }),
            Object::ReverseShell(access, pid) => self.insert(ReverseShellHandle(access, pid)),
            Object::Computer(access) => self.insert(AccessHandle {
                access,
                kind: AccessKind::Computer,
            }),
            Object::File(file) => self.insert(FileHandle(file)),
            Object::Router(router) => self.insert(RouterHandle(router)),
            Object::Metaxploit(meta) => self.insert(MetaxploitHandle(meta)),
            Object::Crypto(access) => self.insert(CryptoHandle(access)),
            Object::AptClient(access) => self.insert(AptClientHandle(access)),
            Object::Blockchain(access) => self.insert(BlockchainHandle(access)),
            Object::SmartAppliance(access) => self.insert(SmartApplianceHandle(access)),
            Object::TrafficNet(access) => self.insert(TrafficNetHandle(access)),
            Object::Service(service) => self.insert(ServiceHandle(service)),
            Object::MetaLib(library) => self.insert(MetaLibHandle(library)),
            Object::NetSession(session) => self.insert(NetSessionHandle(session)),
            Object::DebugLibrary(library) => self.insert(DebugLibraryHandle(library)),
            Object::MetaMail(mail) => self.insert(MetaMailHandle(mail)),
            Object::Wallet(wallet) => self.insert(WalletHandle(wallet)),
            Object::Coin(coin) => self.insert(CoinHandle(coin)),
            Object::SubWallet(sub_wallet) => self.insert(SubWalletHandle(sub_wallet)),
            Object::Ctf(ctf) => self.insert(CtfHandle(ctf)),
        }
    }
}

impl ObjectTableExt for ObjectTableHandle<ComputerId> {
    fn add(&mut self, object: Object) -> ObjectId {
        self.write(|objects| objects.add(object))
    }

    fn apt_client(&self, id: ObjectId) -> Result<Access> {
        self.read(|objects| objects.apt_client(id))
    }

    fn blockchain(&self, id: ObjectId) -> Result<Access> {
        self.read(|objects| objects.blockchain(id))
    }

    fn service(&self, id: ObjectId) -> Result<ServiceRef> {
        self.read(|objects| objects.service(id))
    }

    fn wallet(&self, id: ObjectId) -> Result<WalletId> {
        self.read(|objects| objects.wallet(id))
    }

    fn coin(&self, id: ObjectId) -> Result<CoinRef> {
        self.read(|objects| objects.coin(id))
    }

    fn sub_wallet(&self, id: ObjectId) -> Result<SubWalletRef> {
        self.read(|objects| objects.sub_wallet(id))
    }
}
