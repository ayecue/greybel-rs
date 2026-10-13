use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

const SEPARATOR: char = '.';

/// Definition major and minor version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    /// Major version, incremented for breaking changes.
    pub major: u16,
    /// Additive minor version within a major version.
    pub minor: u16,
}

impl Version {
    #[must_use]
    pub fn new(major: u16, minor: u16) -> Self {
        Self { major, minor }
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}{SEPARATOR}{}", self.major, self.minor)
    }
}

impl FromStr for Version {
    type Err = ();

    fn from_str(text: &str) -> Result<Self, ()> {
        let (major, minor) = text.split_once(SEPARATOR).ok_or(())?;
        let number = |text: &str| {
            if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(());
            }

            text.parse::<u16>().map_err(|_| ())
        };

        Ok(Self::new(number(major)?, number(minor)?))
    }
}

/// Definition version support status.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Status {
    /// Fully supported version.
    #[default]
    Current,
    /// Supported deprecated version. Peers receive the deprecation note on connection.
    Deprecation(String),
    /// Unsupported version. Peers cannot connect with it.
    Unavailable,
}
