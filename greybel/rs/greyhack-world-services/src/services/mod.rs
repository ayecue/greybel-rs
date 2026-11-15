//! Package repositories, service installation and control, and service records.

pub(crate) mod apt;
pub(crate) mod control;
pub(crate) mod install;
mod restart;

pub use apt::Apt;
pub use apt::updates::AptUpdateLine;
pub use control::{ServiceControl, ServiceControlFailure, ServiceStart};
pub use install::records::RecordFailure;
pub use install::service::InstallService;
pub use restart::{ServiceRestart, ServiceRestartFailure, ServiceRestartReport};
