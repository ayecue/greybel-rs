use super::BepInExMajor;
use crate::platform::Platform;

/// A BepInEx archive this tool installs: its version, address and SHA-256 for one platform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BepInExRelease {
    pub major: BepInExMajor,
    pub version: &'static str,
    pub platform: Platform,
    pub file_name: String,
    pub url: String,
    pub sha256: &'static str,
}

impl BepInExRelease {
    const BETA_BUILD_VERSION: &'static str = "6.0.0-be.791+b782da4";
    const BETA_DOWNLOAD_URL: &'static str = "https://builds.bepinex.dev/projects/bepinex_be/791/";
    const BETA_VERSION: &'static str = "6.0.0-be.791";
    const STABLE_DOWNLOAD_URL: &'static str =
        "https://github.com/BepInEx/BepInEx/releases/download/v5.4.23.6/";
    const STABLE_VERSION: &'static str = "5.4.23.6";

    /// The archive of `major` for `platform`.
    #[must_use]
    pub fn of(major: BepInExMajor, platform: Platform) -> Self {
        match (major, platform) {
            (BepInExMajor::Stable, Platform::Windows) => Self::stable(
                platform,
                "BepInEx_win_x64_5.4.23.6.zip",
                "6c57a714a892efb4e9e1fef63d608a3391f30ec984031c465404e4ee381a0755",
            ),
            (BepInExMajor::Stable, Platform::MacOS) => Self::stable(
                platform,
                "BepInEx_macos_universal_5.4.23.6.zip",
                "2e711e21f6570da964fce4654b1976500dbe043f61778bce0008500078564638",
            ),
            (BepInExMajor::Stable, Platform::Linux) => Self::stable(
                platform,
                "BepInEx_linux_x64_5.4.23.6.zip",
                "f8ac87a97fb1ef85451c04ace1fddb7dd7eb0b079c179b989d6bc462c98bd365",
            ),
            (BepInExMajor::Beta, Platform::Windows) => Self::beta(
                platform,
                "win-x64",
                "5dce6c73f760933957f477839a6e28e5d359f196a5cef4a5c90f5a7a73fce82c",
            ),
            (BepInExMajor::Beta, Platform::MacOS) => Self::beta(
                platform,
                "macos-x64",
                "68af2c22f9c23d343a5cdfade005e64aba6850a4f77824041f8cb78491cbca39",
            ),
            (BepInExMajor::Beta, Platform::Linux) => Self::beta(
                platform,
                "linux-x64",
                "9300b9694eb9cbe2b794282e247e06198a26001e729b63021ebecf0c42e82810",
            ),
        }
    }

    fn stable(platform: Platform, file_name: &str, sha256: &'static str) -> Self {
        Self {
            major: BepInExMajor::Stable,
            version: Self::STABLE_VERSION,
            platform,
            file_name: file_name.to_owned(),
            url: format!("{}{file_name}", Self::STABLE_DOWNLOAD_URL),
            sha256,
        }
    }

    fn beta(platform: Platform, build_target: &str, sha256: &'static str) -> Self {
        let file_name = format!(
            "BepInEx-Unity.Mono-{build_target}-{}.zip",
            Self::BETA_BUILD_VERSION
        );
        let url = format!(
            "{}{}",
            Self::BETA_DOWNLOAD_URL,
            file_name.replace('+', "%2B")
        );

        Self {
            major: BepInExMajor::Beta,
            version: Self::BETA_VERSION,
            platform,
            file_name,
            url,
            sha256,
        }
    }
}
