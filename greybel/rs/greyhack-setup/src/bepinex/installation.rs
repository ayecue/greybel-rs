use std::path::Path;

use pelite::PeFile;
use serde::Serialize;

use super::BepInExMajor;
use crate::game::GameLayout;

/// A BepInEx installation detected from the assemblies in `BepInEx/core`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BepInExInstallation {
    pub major: BepInExMajor,
    /// The product version of its core assembly, without build metadata; empty when unreadable.
    pub version: String,
}

impl BepInExInstallation {
    #[must_use]
    pub fn detect(game: &GameLayout) -> Option<Self> {
        let core = game.bepinex_folder().join("core");

        if core.join("BepInEx.Unity.Mono.dll").is_file() {
            return Some(Self {
                major: BepInExMajor::Beta,
                version: Self::product_version(&core.join("BepInEx.Core.dll")),
            });
        }

        let stable_core = core.join("BepInEx.dll");

        stable_core.is_file().then(|| Self {
            major: BepInExMajor::Stable,
            version: Self::product_version(&stable_core),
        })
    }

    /// "BepInEx 5 (5.4.23.6)", or "BepInEx 5" without a version.
    #[must_use]
    pub fn description(&self) -> String {
        let major = self.major.number();

        if self.version.is_empty() {
            format!("BepInEx {major}")
        } else {
            format!("BepInEx {major} ({})", self.version)
        }
    }

    /// The `ProductVersion` of an assembly's version resource, up to a `+`.
    fn product_version(assembly: &Path) -> String {
        let Ok(bytes) = std::fs::read(assembly) else {
            return String::new();
        };
        let version = PeFile::from_bytes(&bytes)
            .ok()
            .and_then(|file| file.resources().ok())
            .and_then(|resources| resources.version_info().ok())
            .and_then(|info| {
                info.translation()
                    .iter()
                    .find_map(|&language| info.value(language, "ProductVersion"))
            })
            .unwrap_or_default();

        version.split('+').next().unwrap_or_default().to_owned()
    }
}
