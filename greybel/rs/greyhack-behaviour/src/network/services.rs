//! Service identities, library mappings, and static metadata.

use greyhack_world_assets::ServiceDefinition;

use crate::LibraryKind;

/// Services available to generated computers in generation order; also accepted by `add_computer`.
pub const COMPUTER_SERVICES: &[ServiceId] = &[
    ServiceId::Ssh,
    ServiceId::Ftp,
    ServiceId::Http,
    ServiceId::Smtp,
    ServiceId::Repository,
    ServiceId::Rshell,
];

/// Public port of the configuration page of each router (`Router.RouterConfig`).
pub const ROUTER_PORT: u16 = 8080;

enum_def! {
    /// Computer service (`ServicioID`) with game numeric IDs as discriminants.
    pub enum ServiceId {
        Ssh = 0,
        Ftp = 1,
        Http = 2,
        Smtp = 3,
        Criminals = 4,
        Employees = 5,
        Students = 6,
        BankAccount = 7,
        Cam = 9,
        Rshell = 10,
        Repository = 11,
        SmartAppliance = 12,
        Adb = 13,
    }
}

impl ServiceId {
    /// Parses `lib<name>.so` or a bare service name. Returns no service for `libsql.so`, which is shared
    /// by four SQL-backed services.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        let name = name.strip_prefix("lib").unwrap_or(name);
        let name = name.strip_suffix(".so").unwrap_or(name);

        Some(match name {
            "ssh" => Self::Ssh,
            "ftp" => Self::Ftp,
            "http" => Self::Http,
            "smtp" => Self::Smtp,
            "criminals" => Self::Criminals,
            "employees" => Self::Employees,
            "students" => Self::Students,
            "bank_account" => Self::BankAccount,
            "cam" => Self::Cam,
            "rshell" => Self::Rshell,
            "repository" => Self::Repository,
            "smartappliance" => Self::SmartAppliance,
            "adb" => Self::Adb,
            _ => return None,
        })
    }

    /// Returns the service name used by `connect_service` and port reports.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Ssh => "ssh",
            Self::Ftp => "ftp",
            Self::Http => "http",
            Self::Smtp => "smtp",
            Self::Criminals => "criminals",
            Self::Employees => "employees",
            Self::Students => "students",
            Self::BankAccount => "bank_account",
            Self::Cam => "cam",
            Self::Rshell => "rshell",
            Self::Repository => "repository",
            Self::SmartAppliance => "smartappliance",
            Self::Adb => "adb",
        }
    }

    /// Returns the port opened by a new `start_service` call.
    #[must_use]
    pub fn default_port(self) -> u16 {
        self.definition().port
    }

    /// Returns the client library filename (`Servicio.GetLibName`). All four SQL-backed services use
    /// `libsql.so`.
    #[must_use]
    pub fn library(self) -> String {
        match self {
            Self::Criminals | Self::Employees | Self::Students | Self::BankAccount => {
                "libsql.so".to_owned()
            }
            _ => format!("lib{}.so", self.name()),
        }
    }

    /// Returns the backing library kind; SQL-backed services use `libsql.so`.
    #[must_use]
    pub const fn library_kind(self) -> LibraryKind {
        match self {
            Self::Ssh => LibraryKind::Ssh,
            Self::Ftp => LibraryKind::Ftp,
            Self::Http => LibraryKind::Http,
            Self::Smtp => LibraryKind::Smtp,
            Self::Criminals | Self::Employees | Self::Students | Self::BankAccount => {
                LibraryKind::Sql
            }
            Self::Cam => LibraryKind::Cam,
            Self::Rshell => LibraryKind::Rshell,
            Self::Repository => LibraryKind::Repository,
            Self::SmartAppliance => LibraryKind::SmartAppliance,
            Self::Adb => LibraryKind::Adb,
        }
    }

    /// Returns the record-viewer program for SQL services that provide one.
    #[must_use]
    pub const fn record_viewer(self) -> Option<&'static str> {
        match self {
            Self::Criminals => Some("PoliceRecord.exe"),
            Self::Employees => Some("EmployeesViewer.exe"),
            Self::Students => Some("StudentsViewer.exe"),
            _ => None,
        }
    }

    /// Returns the process name of the running service daemon.
    #[must_use]
    pub fn daemon(self) -> &'static str {
        &self.definition().daemon
    }

    /// Returns the service configuration path.
    #[must_use]
    pub fn config_path(self) -> &'static str {
        &self.definition().config_path
    }

    /// Returns the service executable path.
    #[must_use]
    pub fn executable_path(self) -> &'static str {
        &self.definition().executable_path
    }

    fn definition(self) -> &'static ServiceDefinition {
        ServiceDefinition::find(self.name()).expect("every supported service is in the game table")
    }
}
