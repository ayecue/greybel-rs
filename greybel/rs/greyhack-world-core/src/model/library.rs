//! Library identity shared by files, ports, and router kernels. Weaknesses and memory zones belong
//! to the libraries domain.

use greyhack_behaviour::LibraryKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LibraryId(pub u32);

/// Kind and version of a library (`VersionsControl`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Library {
    pub kind: LibraryKind,
    pub version: String,
}

impl Library {
    /// Library filename, such as `crypto.so`.
    #[must_use]
    pub fn name(&self) -> String {
        self.kind.file_name()
    }
}
