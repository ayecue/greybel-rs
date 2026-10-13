use crate::definition::Version;
use crate::protocol::{VersionInfo, VersionStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VersionTable {
    pub(crate) version: Version,
    pub(crate) versions: &'static [VersionInfo],
}

impl VersionTable {
    pub(crate) fn fingerprint(&self, minor: u16) -> Option<u32> {
        self.versions
            .get(usize::from(minor))
            .map(|version| version.fingerprint)
    }

    pub(crate) fn status(&self, minor: u16) -> VersionStatus {
        self.versions
            .get(usize::from(minor))
            .map_or(VersionStatus::Current, |version| version.status)
    }
}
