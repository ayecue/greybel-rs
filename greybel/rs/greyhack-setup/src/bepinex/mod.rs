mod installation;
mod installer;
mod launcher;
mod release;

use serde::Serialize;

pub use self::installation::BepInExInstallation;
pub use self::installer::BepInExInstaller;
pub use self::launcher::BepInExLauncher;
pub use self::release::BepInExRelease;

/// The BepInEx line: 5 is the stable release, 6 a beta build.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(into = "u8")]
pub enum BepInExMajor {
    Stable,
    Beta,
}

impl BepInExMajor {
    pub const ALL: [Self; 2] = [Self::Stable, Self::Beta];

    #[must_use]
    pub fn number(self) -> u8 {
        match self {
            Self::Stable => 5,
            Self::Beta => 6,
        }
    }

    #[must_use]
    pub fn from_number(number: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|major| major.number() == number)
    }
}

impl std::str::FromStr for BepInExMajor {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, String> {
        value
            .parse()
            .ok()
            .and_then(Self::from_number)
            .ok_or_else(|| "must be 5 or 6".to_owned())
    }
}

impl From<BepInExMajor> for u8 {
    fn from(major: BepInExMajor) -> Self {
        major.number()
    }
}
