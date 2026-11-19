use std::cmp::Ordering;
use std::collections::BTreeMap;

use greyhack_setup::{Downloader, SetupResult};
use serde::{Deserialize, Serialize};

/// Where releases of `greybel` are published: `<url>/latest.json` describes the newest one, `<url>/<version>/` has
/// its archives.
pub struct ReleaseFeed {
    url: String,
}

/// `latest.json`: the newest release and its archive for each Rust target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LatestRelease {
    pub version: String,
    pub tag: String,
    pub assets: BTreeMap<String, ReleaseAsset>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseAsset {
    pub url: String,
    pub sha256: String,
    pub size: u64,
}

impl ReleaseFeed {
    const URL: &'static str = "https://greybel-assets.fra1.digitaloceanspaces.com/releases/greybel";
    /// The environment variable naming another feed, for tests and mirrors.
    pub const URL_VARIABLE: &'static str = "GREYBEL_UPDATE_URL";

    /// `GREYBEL_UPDATE_URL`, else the release folder on DigitalOcean Spaces.
    pub fn new() -> Self {
        let url = std::env::var(Self::URL_VARIABLE)
            .ok()
            .filter(|url| !url.is_empty())
            .unwrap_or_else(|| Self::URL.to_owned());

        Self {
            url: url.trim_end_matches('/').to_owned(),
        }
    }

    pub fn latest(&self, downloader: &Downloader) -> SetupResult<LatestRelease> {
        let url = format!("{}/latest.json", self.url);
        let text = downloader.read_text(&url)?;

        serde_json::from_str(&text).map_err(|error| greyhack_setup::SetupError::Download {
            url,
            message: format!("it isn't a release description: {error}"),
        })
    }
}

impl LatestRelease {
    pub fn is_newer_than(&self, version: &str) -> bool {
        ReleaseVersion::compare(&self.version, version) == Ordering::Greater
    }
}

/// Compares `major.minor.patch` versions; a version with a `-` suffix comes before the same version without.
pub struct ReleaseVersion;

impl ReleaseVersion {
    pub fn compare(left: &str, right: &str) -> Ordering {
        let parse = |version: &str| {
            let (numbers, suffix) = version.trim_start_matches('v').split_once('-').map_or(
                (version.trim_start_matches('v'), None),
                |(numbers, suffix)| (numbers, Some(suffix.to_owned())),
            );
            let numbers: Vec<u64> = numbers
                .split('.')
                .map(|number| number.parse().unwrap_or(0))
                .collect();

            (numbers, suffix)
        };
        let (left_numbers, left_suffix) = parse(left);
        let (right_numbers, right_suffix) = parse(right);

        left_numbers
            .cmp(&right_numbers)
            .then_with(|| match (left_suffix, right_suffix) {
                (None, Some(_)) => Ordering::Greater,
                (Some(_), None) => Ordering::Less,
                (left, right) => left.cmp(&right),
            })
    }
}
