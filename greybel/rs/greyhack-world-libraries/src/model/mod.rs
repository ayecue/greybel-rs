//! Library entities, exploit offers, debug state, and Metaxploit session results.

pub mod debug;
pub mod exploit;
pub mod libraries;
pub mod network;

pub use debug::{
    DebugAccount, DebugChallenge, DebugLibraryState, ZeroDay, ZeroDayObject, ZeroDayObjectKind,
};
pub use exploit::{Exploit, ExploitOffer};
pub use greyhack_behaviour::SmartApplianceSettings;
pub use libraries::{Library, MemoryZone, ReverseShell, Weakness};
pub use network::{NetUseFailure, NetUseSession, RshellFailure};

/// Computer generation draft carrying full library entities.
pub type ComputerDraft = greyhack_world_core::ComputerDraft<Library, SmartApplianceSettings>;
/// File-system generation draft carrying full library entities.
pub type FileSystemDraft = greyhack_world_core::FileSystemDraft<Library>;
/// Node generation draft carrying full library entities.
pub type NodeDraft = greyhack_world_core::NodeDraft<Library>;
/// Router generation draft carrying full library entities.
pub type RouterDraft = greyhack_world_core::RouterDraft<Library, SmartApplianceSettings>;
