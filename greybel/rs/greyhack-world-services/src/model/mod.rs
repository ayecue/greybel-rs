//! Package, service-configuration, and SQL-service record entities.

pub(crate) mod apt;
pub(crate) mod records;
pub(crate) mod service_config;

pub use apt::{AptFailure, Package, PackageEntry, Repository};
pub use records::{
    Crime, FacePart, Grade, Offense, Record, RecordDatabase, RecordKind, Subject, Vector,
};
pub use service_config::{
    DEFAULT_DECODE_PATH, DEFAULT_ENCODE_PATH, DEFAULT_REPOSITORY_PATH, HttpConfig,
    RepositoryConfig, SoftwareInfo, SshdConfig,
};
