//! Message-hook plugin protocol bindings for Grey Hack, with compressed-payload and notification-routing helpers.

mod build;
mod context;
mod payload;
mod recipient;
mod window;

pub use build::{BuildState, ScriptType};
pub use context::{ContextBreakpoint, DebugEntry, DebugFrame, DebugValue, StackItem, StepMode};
use duplex_core::{Metadata, Tag};
pub use payload::Payload;
pub use window::WindowPidKind;

/// Client protocol messages and channel bindings.
pub mod greyhack {
    duplex_core::include_protocol!("$PROTOCOLS/greyhack", major = 1, role = client);
}

/// Provider-side bindings for mock plugin servers in tests.
#[cfg(feature = "provider")]
pub mod provider {
    duplex_core::include_protocol!("$PROTOCOLS/greyhack", major = 1, role = provider);
}

pub use greyhack::GreybelNotification;

/// Default port from the plugin's `Port` setting.
pub const DEFAULT_PORT: u16 = 7777;
pub const DEFAULT_HOST: &str = "127.0.0.1";

/// Metadata tag for the protocol's Grey Hack version.
pub const GAME_VERSION_TAG: &str = "game_version";
/// Field tag for window-PID metadata.
pub const PID_TAG: &str = "pid";

/// Reads protocol `game_version` metadata, defaulting to an empty string.
#[must_use]
pub fn game_version() -> &'static str {
    Tag::find(<greyhack::GreyHack as Metadata>::TAGS, GAME_VERSION_TAG)
        .and_then(Tag::text)
        .unwrap_or_default()
}
