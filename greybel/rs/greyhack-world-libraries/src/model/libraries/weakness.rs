//! Library memory zones and exploitation weaknesses (`Library`). The generators are in
//! `greyhack-world`.

use greyhack_behaviour::{LibraryKind, Requirement, Reward};

/// Memory-address exploit definition (`Vulnerabilidad`).
#[derive(Debug, Clone, PartialEq)]
pub struct Weakness {
    /// Whether exploitation originates from another computer (`Vulnerabilidad.isRemote`).
    pub remote: bool,
    pub unsec_value: String,
    pub details: String,
    pub requirements: Vec<Requirement>,
    pub reward: Reward,
    /// Post-exploitation account: `guest`, `normal_user`, or `root`.
    pub user: String,
    pub required_lib: Option<LibraryKind>,
    pub req_lib_version: String,
    /// Minimum Metaxploit version required for exploitation.
    pub metaxploit_version: String,
    pub num_register_users: u32,
    pub num_port_forward: u32,
    pub num_conn_gateway: u32,
    pub path_exist: String,
    pub random_path: String,
}

/// Memory zone containing zero or more weaknesses (`MemoryZone`).
#[derive(Debug, Clone, PartialEq)]
pub struct MemoryZone {
    pub address: String,
    pub hide: bool,
    pub weaknesses: Vec<Weakness>,
}

/// Library version and exploitation zones (`Library`), shared by identical files, ports, and router
/// kernels.
#[derive(Debug, Clone, PartialEq)]
pub struct Library {
    pub kind: LibraryKind,
    pub version: String,
    pub zones: Vec<MemoryZone>,
}

impl Library {
    /// Library filename, such as `crypto.so`.
    #[must_use]
    pub fn name(&self) -> String {
        self.kind.file_name()
    }

    /// Looks up `address` and `unsec_value`, returning the matching weakness if present.
    #[must_use]
    pub fn weakness(&self, address: &str, unsec_value: &str) -> Option<&Weakness> {
        self.zones
            .iter()
            .find(|zone| zone.address == address)?
            .weaknesses
            .iter()
            .find(|weakness| weakness.unsec_value == unsec_value)
    }
}
