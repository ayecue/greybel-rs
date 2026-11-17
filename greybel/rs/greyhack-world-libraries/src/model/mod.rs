//! Library entities, exploit offers, debug state, and Metaxploit session results.

use greyhack_behaviour::SmartApplianceSettings;

pub mod debug;
pub mod exploit;
pub mod libraries;
pub mod network;
pub mod zero_day_rumor;

pub use debug::{DebugChallenge, DebugLibraryState, ZeroDay, ZeroDayObject, ZeroDayObjectKind};
pub use exploit::{Exploit, ExploitOffer};
pub use libraries::{Library, MemoryZone, ReverseShell, Weakness};
pub use network::{NetUseFailure, NetUseSession, RshellFailure};
pub use zero_day_rumor::{
    CREDENTIALS_IN_USE, DebugCredentials, DebugNetworkBinding, DebugOffer, DebugPurchase,
    INCORRECT_OFFER_CREDENTIALS, NpcDebugCredentials, OFFER_NOT_AVAILABLE, ONE_OFFER_PER_PLAYER,
    SALE_CLOSED, USED_OFFER_CREDENTIALS, ZeroDayPlayerState, ZeroDayRumor,
};

/// Computer generation draft carrying full library entities.
pub type ComputerDraft = greyhack_world_core::ComputerDraft<Library, SmartApplianceSettings>;
/// File-system generation draft carrying full library entities.
pub type FileSystemDraft = greyhack_world_core::FileSystemDraft<Library>;
/// Node generation draft carrying full library entities.
pub type NodeDraft = greyhack_world_core::NodeDraft<Library>;
/// Router generation draft carrying full library entities.
pub type RouterDraft = greyhack_world_core::RouterDraft<Library, SmartApplianceSettings>;
