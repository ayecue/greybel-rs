mod config;
mod installer;
mod release;

use std::net::{Ipv4Addr, TcpListener};

pub use self::config::HookConfig;
pub use self::installer::HookInstaller;
pub use self::release::{HookRelease, ReleaseAsset};
use crate::bepinex::BepInExMajor;

/// The message hook plugin: its BepInEx GUID and its assembly for each BepInEx major.
pub struct MessageHook;

impl MessageHook {
    pub const DEFAULT_PORT: u16 = 7777;
    pub const DEFAULT_REPOSITORY: &'static str = "ayecue/greybel-rs";
    pub const GUID: &'static str = "GreyHackMessageHook";
    /// The message hook release version whose game message protocol matches this build.
    pub const VERSION: &'static str = env!("MESSAGE_HOOK_VERSION");

    /// The assembly name for `major`, without `.dll`.
    #[must_use]
    pub fn assembly_name(major: BepInExMajor) -> &'static str {
        match major {
            BepInExMajor::Stable => "GreyHackMessageHook5",
            BepInExMajor::Beta => "GreyHackMessageHook",
        }
    }

    /// The release tag of [`Self::VERSION`].
    #[must_use]
    pub fn release_tag() -> String {
        format!("greyhack-message-hook/v{}", Self::VERSION)
    }

    /// Whether something on this machine already listens on `port`.
    #[must_use]
    pub fn is_port_in_use(port: u16) -> bool {
        TcpListener::bind((Ipv4Addr::LOCALHOST, port)).is_err()
    }
}
