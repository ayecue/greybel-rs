//! Updates `greybel` from its releases on DigitalOcean Spaces, and reminds of newer versions.

mod feed;
mod install;
mod reminder;

use greyhack_setup::Downloader;

use self::feed::LatestRelease;
pub use self::feed::ReleaseFeed;
pub use self::install::SelfUpdate;
pub use self::reminder::UpdateReminder;
use crate::report::Failure;

/// This program's version, and the newest release.
pub struct UpdateCheck;

impl UpdateCheck {
    pub const USER_AGENT: &'static str = concat!("greybel/", env!("CARGO_PKG_VERSION"));
    pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    pub fn latest() -> Result<LatestRelease, Failure> {
        let downloader = Downloader::with_user_agent(std::env::temp_dir(), Self::USER_AGENT);

        ReleaseFeed::new()
            .latest(&downloader)
            .map_err(|error| Failure::new("Update error", error))
    }
}
