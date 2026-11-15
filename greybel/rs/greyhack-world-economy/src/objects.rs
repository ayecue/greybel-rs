//! Economy script-object handles and their game identity rules.

use std::any::Any;

use greyhack_world_core::{Access, CoinId, ComputerId, WorldError};
use greyhack_world_programs::protocol::ObjectId;
use greyhack_world_programs::{
    Handle, ObjectClass, ObjectIdentity, ObjectReach, ObjectTable as TreeObjects, ObjectTableHandle,
};

use crate::WalletId;
use crate::routes::Result;

/// Coin `coin` accessed through the blockchain library on `computer`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CoinRef {
    /// Coin opened by the script.
    pub coin: CoinId,
    /// Computer hosting the blockchain library.
    pub computer: ComputerId,
}

/// Sub-wallet keyed by coin `coin` and user `user`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubWalletRef {
    /// Coin of the sub-wallet.
    pub coin: CoinId,
    /// Sub-wallet user name.
    pub user: String,
}

/// Blockchain library accessed with `access`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BlockchainHandle(pub Access);

impl Handle<ComputerId> for BlockchainHandle {
    const CLASS: ObjectClass = ObjectClass::Blockchain;

    fn identity(&self) -> ObjectIdentity {
        ObjectIdentity::of(Self::CLASS, self)
    }

    fn same_identity(&self, other: &dyn Any) -> bool {
        other.downcast_ref::<Self>() == Some(self)
    }

    fn reach(&self) -> Option<ObjectReach<ComputerId>> {
        Some(ObjectReach::Computer(self.0.computer))
    }
}

/// Authenticated wallet.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WalletHandle(pub WalletId);

impl Handle<ComputerId> for WalletHandle {
    const CLASS: ObjectClass = ObjectClass::Wallet;

    fn identity(&self) -> ObjectIdentity {
        ObjectIdentity::of(Self::CLASS, self)
    }

    fn same_identity(&self, other: &dyn Any) -> bool {
        other.downcast_ref::<Self>() == Some(self)
    }

    fn reach(&self) -> Option<ObjectReach<ComputerId>> {
        None
    }
}

/// Authenticated coin. Coin identity excludes the computer used to open it (`HelperScript.AddCoin`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CoinHandle(pub CoinRef);

impl Handle<ComputerId> for CoinHandle {
    const CLASS: ObjectClass = ObjectClass::Coin;

    fn identity(&self) -> ObjectIdentity {
        ObjectIdentity::of(Self::CLASS, &self.0.coin)
    }

    fn same_identity(&self, other: &dyn Any) -> bool {
        other
            .downcast_ref::<Self>()
            .is_some_and(|other| self.0.coin == other.0.coin)
    }

    fn reach(&self) -> Option<ObjectReach<ComputerId>> {
        None
    }
}

/// Sub-wallet object handle.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubWalletHandle(pub SubWalletRef);

impl Handle<ComputerId> for SubWalletHandle {
    const CLASS: ObjectClass = ObjectClass::SubWallet;

    fn identity(&self) -> ObjectIdentity {
        ObjectIdentity::of(Self::CLASS, self)
    }

    fn same_identity(&self, other: &dyn Any) -> bool {
        other.downcast_ref::<Self>() == Some(self)
    }

    fn reach(&self) -> Option<ObjectReach<ComputerId>> {
        None
    }
}

/// Typed economy-object lookup and registration on a tree's object table.
pub trait ObjectTableExt {
    /// Returns blockchain access.
    fn blockchain(&self, id: ObjectId) -> Result<Access>;
    /// Returns wallet identity.
    fn wallet(&self, id: ObjectId) -> Result<WalletId>;
    /// Returns coin access.
    fn coin(&self, id: ObjectId) -> Result<CoinRef>;
    /// Returns sub-wallet access.
    fn sub_wallet(&self, id: ObjectId) -> Result<SubWalletRef>;
    /// Registers an authenticated wallet.
    fn add_wallet(&mut self, wallet: WalletId) -> ObjectId;
    /// Registers an authenticated coin.
    fn add_coin(&mut self, coin: CoinRef) -> ObjectId;
    /// Registers a sub-wallet.
    fn add_sub_wallet(&mut self, sub_wallet: SubWalletRef) -> ObjectId;
}

impl ObjectTableExt for TreeObjects<ComputerId> {
    fn blockchain(&self, id: ObjectId) -> Result<Access> {
        Ok(self.get::<BlockchainHandle, WorldError>(id)?.0.clone())
    }

    fn wallet(&self, id: ObjectId) -> Result<WalletId> {
        Ok(self.get::<WalletHandle, WorldError>(id)?.0)
    }

    fn coin(&self, id: ObjectId) -> Result<CoinRef> {
        Ok(self.get::<CoinHandle, WorldError>(id)?.0)
    }

    fn sub_wallet(&self, id: ObjectId) -> Result<SubWalletRef> {
        Ok(self.get::<SubWalletHandle, WorldError>(id)?.0.clone())
    }

    fn add_wallet(&mut self, wallet: WalletId) -> ObjectId {
        self.insert(WalletHandle(wallet))
    }

    fn add_coin(&mut self, coin: CoinRef) -> ObjectId {
        self.insert(CoinHandle(coin))
    }

    fn add_sub_wallet(&mut self, sub_wallet: SubWalletRef) -> ObjectId {
        self.insert(SubWalletHandle(sub_wallet))
    }
}

impl ObjectTableExt for ObjectTableHandle<ComputerId> {
    fn blockchain(&self, id: ObjectId) -> Result<Access> {
        self.read(|objects| ObjectTableExt::blockchain(objects, id))
    }

    fn wallet(&self, id: ObjectId) -> Result<WalletId> {
        self.read(|objects| ObjectTableExt::wallet(objects, id))
    }

    fn coin(&self, id: ObjectId) -> Result<CoinRef> {
        self.read(|objects| ObjectTableExt::coin(objects, id))
    }

    fn sub_wallet(&self, id: ObjectId) -> Result<SubWalletRef> {
        self.read(|objects| ObjectTableExt::sub_wallet(objects, id))
    }

    fn add_wallet(&mut self, wallet: WalletId) -> ObjectId {
        self.write(|objects| ObjectTableExt::add_wallet(objects, wallet))
    }

    fn add_coin(&mut self, coin: CoinRef) -> ObjectId {
        self.write(|objects| ObjectTableExt::add_coin(objects, coin))
    }

    fn add_sub_wallet(&mut self, sub_wallet: SubWalletRef) -> ObjectId {
        self.write(|objects| ObjectTableExt::add_sub_wallet(objects, sub_wallet))
    }
}
