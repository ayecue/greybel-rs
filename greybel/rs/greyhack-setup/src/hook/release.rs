use std::path::PathBuf;

use serde_json::Value;

use super::MessageHook;
use crate::bepinex::BepInExMajor;
use crate::download::Downloader;
use crate::error::{SetupError, SetupResult};

/// A GitHub release of the message hook: its tag and its files with their SHA-256 digests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookRelease {
    pub tag: String,
    pub assets: Vec<ReleaseAsset>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseAsset {
    pub name: String,
    pub url: String,
    /// GitHub's SHA-256 digest of the file; `None` for releases from before GitHub recorded digests.
    pub sha256: Option<String>,
}

impl HookRelease {
    /// The release `tag` of `repository`, or its latest release without a tag.
    pub fn find(downloader: &Downloader, repository: &str, tag: Option<&str>) -> SetupResult<Self> {
        let path = match tag {
            Some(tag) => format!("tags/{}", tag.replace('/', "%2F")),
            None => "latest".to_owned(),
        };
        let unreadable = |message: String| SetupError::ReleasesUnreadable {
            repository: repository.to_owned(),
            message,
        };
        let mut response = downloader
            .agent()
            .get(format!(
                "https://api.github.com/repos/{repository}/releases/{path}"
            ))
            .header("Accept", "application/vnd.github+json")
            .call()
            .map_err(|error| unreadable(error.to_string()))?;
        let status = response.status().as_u16();

        if !response.status().is_success() {
            return Err(match tag {
                Some(tag) => SetupError::ReleaseTagMissing {
                    tag: tag.to_owned(),
                    repository: repository.to_owned(),
                    status,
                },
                None => SetupError::ReleaseMissing {
                    repository: repository.to_owned(),
                    status,
                },
            });
        }

        let text = response
            .body_mut()
            .read_to_string()
            .map_err(|error| unreadable(error.to_string()))?;

        Self::parse(&text).ok_or_else(|| unreadable("the response isn't a release".to_owned()))
    }

    /// A release in GitHub's JSON.
    #[must_use]
    pub fn parse(json: &str) -> Option<Self> {
        let release: Value = serde_json::from_str(json).ok()?;
        let assets = release["assets"]
            .as_array()?
            .iter()
            .filter_map(|asset| {
                Some(ReleaseAsset {
                    name: asset["name"].as_str()?.to_owned(),
                    url: asset["browser_download_url"].as_str()?.to_owned(),
                    sha256: asset["digest"]
                        .as_str()
                        .and_then(|digest| digest.strip_prefix("sha256:"))
                        .map(str::to_owned),
                })
            })
            .collect();

        Some(Self {
            tag: release["tag_name"].as_str()?.to_owned(),
            assets,
        })
    }

    /// The hook build for `major`, which must have a digest.
    pub fn asset(&self, major: BepInExMajor) -> SetupResult<&ReleaseAsset> {
        let name = format!("{}.dll", MessageHook::assembly_name(major));
        let asset = self
            .assets
            .iter()
            .find(|asset| asset.name == name)
            .ok_or_else(|| SetupError::ReleaseAssetMissing {
                tag: self.tag.clone(),
                name: name.clone(),
            })?;

        if asset.sha256.is_none() {
            return Err(SetupError::ReleaseChecksumMissing {
                tag: self.tag.clone(),
                name,
            });
        }

        Ok(asset)
    }

    /// Downloads the hook build for `major` into the cache folder of this release's tag.
    pub fn fetch(&self, downloader: &Downloader, major: BepInExMajor) -> SetupResult<PathBuf> {
        let asset = self.asset(major)?;

        downloader.fetch(
            &asset.url,
            &self.tag,
            &asset.name,
            asset.sha256.as_deref().unwrap_or_default(),
        )
    }
}
