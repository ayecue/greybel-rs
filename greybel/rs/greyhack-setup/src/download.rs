use std::fmt::Write as _;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use sha2::{Digest, Sha256};
use ureq::Agent;

use crate::error::{SetupError, SetupResult};
use crate::platform::Platform;

/// Downloads files into a cache folder and checks them against their SHA-256; a cached file with the right
/// checksum isn't downloaded again.
pub struct Downloader {
    agent: Agent,
    cache_folder: PathBuf,
}

impl Downloader {
    const USER_AGENT: &'static str = concat!("greyhack-setup/", env!("CARGO_PKG_VERSION"));

    #[must_use]
    pub fn new(cache_folder: PathBuf) -> Self {
        Self::with_user_agent(cache_folder, Self::USER_AGENT)
    }

    /// A downloader whose requests name `user_agent`, such as `greybel/0.2.0`.
    #[must_use]
    pub fn with_user_agent(cache_folder: PathBuf, user_agent: &str) -> Self {
        let agent = Agent::config_builder()
            .http_status_as_error(false)
            .timeout_connect(Some(Duration::from_secs(30)))
            .timeout_recv_body(Some(Duration::from_secs(300)))
            .user_agent(user_agent)
            .build()
            .into();

        Self {
            agent,
            cache_folder,
        }
    }

    /// `greyhack-setup` in the user's cache folder, else in the system's temporary folder.
    #[must_use]
    pub fn default_cache_folder(platform: Platform) -> PathBuf {
        platform
            .cache_folder()
            .unwrap_or_else(std::env::temp_dir)
            .join("greyhack-setup")
    }

    #[must_use]
    pub fn agent(&self) -> &Agent {
        &self.agent
    }

    #[must_use]
    pub fn cache_folder(&self) -> &Path {
        &self.cache_folder
    }

    pub fn read_text(&self, url: &str) -> SetupResult<String> {
        let failure = |message: String| SetupError::Download {
            url: url.to_owned(),
            message,
        };
        let mut response = self
            .agent
            .get(url)
            .call()
            .map_err(|error| failure(error.to_string()))?;

        if !response.status().is_success() {
            return Err(SetupError::DownloadStatus {
                url: url.to_owned(),
                status: response.status().as_u16(),
            });
        }

        response
            .body_mut()
            .read_to_string()
            .map_err(|error| failure(error.to_string()))
    }

    /// The file `url` names, stored as `folder/file_name` in the cache, with the SHA-256 `sha256`.
    pub fn fetch(
        &self,
        url: &str,
        folder: &str,
        file_name: &str,
        sha256: &str,
    ) -> SetupResult<PathBuf> {
        let folder = self.cache_folder.join(folder);
        let path = folder.join(file_name);

        if path.is_file()
            && Self::sha256_of(&path).is_ok_and(|actual| actual.eq_ignore_ascii_case(sha256))
        {
            return Ok(path);
        }

        std::fs::create_dir_all(&folder).map_err(SetupError::io(&folder))?;

        let partial = folder.join(format!("{file_name}.part"));
        let result = self.download(url, &partial).and_then(|()| {
            let actual = Self::sha256_of(&partial)?;

            if actual.eq_ignore_ascii_case(sha256) {
                std::fs::rename(&partial, &path).map_err(SetupError::io(&path))
            } else {
                Err(SetupError::ChecksumMismatch {
                    url: url.to_owned(),
                    sha256: sha256.to_owned(),
                    actual,
                })
            }
        });
        let _ = std::fs::remove_file(&partial);

        result.map(|()| path)
    }

    /// The lowercase hexadecimal SHA-256 of the file at `path`.
    pub fn sha256_of(path: &Path) -> SetupResult<String> {
        let mut file = File::open(path).map_err(SetupError::io(path))?;
        let mut hasher = Sha256::new();

        io::copy(&mut file, &mut hasher).map_err(SetupError::io(path))?;

        Ok(hasher
            .finalize()
            .iter()
            .fold(String::with_capacity(64), |mut hex, byte| {
                let _ = write!(hex, "{byte:02x}");

                hex
            }))
    }

    fn download(&self, url: &str, target: &Path) -> SetupResult<()> {
        let failure = |message: String| SetupError::Download {
            url: url.to_owned(),
            message,
        };
        let mut response = self
            .agent
            .get(url)
            .call()
            .map_err(|error| failure(error.to_string()))?;
        let status = response.status().as_u16();

        if !response.status().is_success() {
            return Err(SetupError::DownloadStatus {
                url: url.to_owned(),
                status,
            });
        }

        let mut file = File::create(target).map_err(SetupError::io(target))?;
        let mut body = response.body_mut().as_reader();

        io::copy(&mut body, &mut file).map_err(|error| failure(error.to_string()))?;

        Ok(())
    }
}
