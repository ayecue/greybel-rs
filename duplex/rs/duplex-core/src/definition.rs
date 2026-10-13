duplex_macros::include_types!("$DUPLEX/core", major = 1, core = crate);

/// Core protocol version supported by this crate.
pub const CORE: Version = Version { major: 1, minor: 0 };

impl FrameKind {
    /// Frame-header byte for this kind.
    pub(crate) const fn byte(self) -> u8 {
        self as u8
    }
}

impl Version {
    pub(crate) fn is_within(self, newer: Self) -> bool {
        self.major == newer.major && self.minor <= newer.minor
    }

    pub(crate) fn lower_minor_version(self, other: Self) -> Self {
        Self {
            major: self.major,
            minor: self.minor.min(other.minor),
        }
    }
}
