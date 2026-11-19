//! Setup operations shared by greyhack-setup, `greybel setup` and the VS Code extension: locate the game, install
//! pinned BepInEx 5 or 6 archives, install the message hook from GitHub or a file, and configure the hook's port.

pub mod bepinex;
#[cfg(feature = "cli")]
pub mod cli;
mod download;
mod error;
pub mod game;
pub mod hook;
mod platform;
mod setup;
mod status;

pub use self::download::Downloader;
pub use self::error::{SetupError, SetupResult};
pub use self::platform::Platform;
pub use self::setup::{BepInExSource, HookSource, Setup};
pub use self::status::{GameStatus, Readiness};
