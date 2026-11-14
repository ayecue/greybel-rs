//! Data types of computers and networks with entity-local methods, and the IDs every domain uses.
//! Cross-entity behavior lives in [`crate::systems`].

/// Numeric entity ID representation for protocols and external callers.
macro_rules! id_numbers {
    ($($id:ident),*) => {$(
        impl $id {
            /// Converts `number` to a typed entity ID.
            #[must_use]
            pub const fn new(number: u32) -> Self {
                Self(number)
            }

            /// Converts a typed ID to its numeric representation.
            #[must_use]
            pub const fn number(self) -> u32 {
                self.0
            }
        }
    )*};
}

pub mod computer;
mod library;
pub mod network;
mod person;
mod trace_kind;

pub use computer::failure::{FileFailure, NetCardFailure, ProcessFailure, UserFailure};
pub use computer::filesystem::{
    CopyReport, DraftKind, FileSystemDraft, Node, NodeDraft, NodeId, NodeKind,
};
pub use computer::startup::StartupFailure;
pub use computer::user::User;
pub use computer::{
    Computer, ComputerDraft, ComputerId, Cpu, Gpu, HardDisk, Hardware, LanPlace, MiningHardware,
    Motherboard, NetCard, Part, Port, PortForward, PowerSupply, Process, Ram, STANDARD_DISK_MB,
    STANDARD_RAM_MB, SavedNetwork, SavedWifiFailure,
};
pub use library::{Library, LibraryId};
pub use network::failure::{ConnectionFailure, LanFailure, LoginFailure, ServiceFailure};
pub use network::router::{Contact, NetworkDraft, Router, RouterDraft, RouterId};
pub use network::{Network, NetworkId};
pub use person::{Gender, Person, PersonId};
pub use trace_kind::TraceKind;

/// Coin ID, which the mining work of the world refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CoinId(pub u32);

id_numbers!(
    ComputerId, RouterId, NodeId, CoinId, PersonId, NetworkId, LibraryId
);
