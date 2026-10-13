//! Library identities, filenames, remote-access rules, and exploit metadata.

use crate::ServiceId;

/// Folder the service libraries of a computer are in.
pub const LIBRARY_FOLDER: &str = "/lib";
/// Path of the router kernel library, the library of a router that its port table needs
/// (`Router.kernel_router`).
pub const KERNEL_ROUTER_PATH: &str = "/lib/kernel_router.so";

/// Returns the path of the library file `file_name` in [`LIBRARY_FOLDER`].
#[must_use]
pub fn library_path(file_name: impl AsRef<str>) -> String {
    format!("{LIBRARY_FOLDER}/{}", file_name.as_ref())
}

enum_def! {
    /// Result of exploiting a weakness (`Library.Vulnerabilidad.Reward`).
    pub enum Reward {
        Shell = 0,
        RandomFolder = 1,
        ChangePass = 2,
        Computer = 3,
        FirewallDisable = 4,
        OverrideSettings = 5,
        TrafficLightControl = 6,
    }
}

enum_def! {
    /// Condition required to exploit a weakness (`Library.Vulnerabilidad.Required`).
    pub enum Requirement {
        Library = 0,
        NumberUsersRegister = 1,
        AnyActiveUser = 2,
        RootActiveUser = 3,
        LocalNetwork = 4,
        PortForward = 5,
        ConnGateway = 6,
        GuestActiveUser = 7,
        PathExist = 8,
    }
}

enum_def! {
    /// Library identity (`VersionsControl.IdLib`), including its filename and the version exposed by
    /// files, ports, or routers.
    pub enum LibraryKind {
        Ssh = 0,
        Ftp = 1,
        Http = 2,
        Sql = 3,
        Smtp = 4,
        Cam = 5,
        Rshell = 6,
        Repository = 7,
        Blockchain = 8,
        Adb = 9,
        SmartAppliance = 10,
        KernelRouter = 11,
        AptClient = 12,
        Metaxploit = 13,
        Crypto = 14,
        KernelModule = 15,
        Init = 16,
        Net = 17,
        TrafficNet = 18,
    }
}

impl LibraryKind {
    /// Returns the filename without `.so`, such as `crypto` or `libssh`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Init => "init",
            Self::KernelModule => "kernel_module",
            Self::Net => "net",
            Self::AptClient => "aptclient",
            Self::Crypto => "crypto",
            Self::Metaxploit => "metaxploit",
            Self::Blockchain => "blockchain",
            Self::KernelRouter => "kernel_router",
            Self::Ssh => "libssh",
            Self::Ftp => "libftp",
            Self::Http => "libhttp",
            Self::Sql => "libsql",
            Self::Smtp => "libsmtp",
            Self::Cam => "libcam",
            Self::Rshell => "librshell",
            Self::Repository => "librepository",
            Self::Adb => "libadb",
            Self::SmartAppliance => "libsmartappliance",
            Self::TrafficNet => "libtrafficnet",
        }
    }

    /// Returns the library filename, such as `crypto.so`.
    #[must_use]
    pub fn file_name(self) -> String {
        format!("{}.so", self.name())
    }

    /// Looks up a library by `file_name`.
    #[must_use]
    pub fn of_file(file_name: &str) -> Option<Self> {
        let name = file_name.strip_suffix(".so")?;

        Self::ALL.iter().copied().find(|kind| kind.name() == name)
    }

    /// Checks whether weaknesses are remotely accessible (`Library.IsRemoteLibrary`). This applies to
    /// every library through `aptclient.so`.
    #[must_use]
    pub const fn is_remote(self) -> bool {
        self.code() <= Self::AptClient.code()
    }

    /// Returns the role used to generate weaknesses.
    #[must_use]
    pub const fn role(self) -> LibraryRole {
        match self {
            Self::KernelRouter => LibraryRole::Router,
            _ => LibraryRole::Service,
        }
    }
}

/// Role that determines weakness generation and network requirements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryRole {
    /// Service library such as `libssh.so`; its weaknesses can require network conditions.
    Service,
    /// `kernel_router.so`; generated weaknesses always include a guest shell or computer weakness.
    Router,
}

/// Script object type `include_lib` gives a library kind (`PlayerIntrinsics.include_lib`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryClass {
    /// `metaxploit.so`.
    Metaxploit,
    /// `crypto.so`.
    Crypto,
    /// `aptclient.so`.
    AptClient,
    /// `blockchain.so`.
    Blockchain,
    /// `libsmartappliance.so`.
    SmartAppliance,
    /// `libtrafficnet.so`.
    TrafficNet,
    /// A service library (SSH, FTP, HTTP, rshell, repository).
    Service(ServiceId),
}

impl LibraryClass {
    /// Maps `kind` to its object type, or returns `None` for kinds `include_lib` gives no object
    /// (`PlayerIntrinsics.include_lib`: only the SSH, FTP, HTTP, rshell and repository libraries are
    /// services).
    #[must_use]
    pub const fn of(kind: LibraryKind) -> Option<Self> {
        Some(match kind {
            LibraryKind::Metaxploit => Self::Metaxploit,
            LibraryKind::Crypto => Self::Crypto,
            LibraryKind::AptClient => Self::AptClient,
            LibraryKind::Blockchain => Self::Blockchain,
            LibraryKind::SmartAppliance => Self::SmartAppliance,
            LibraryKind::TrafficNet => Self::TrafficNet,
            LibraryKind::Ssh => Self::Service(ServiceId::Ssh),
            LibraryKind::Ftp => Self::Service(ServiceId::Ftp),
            LibraryKind::Http => Self::Service(ServiceId::Http),
            LibraryKind::Rshell => Self::Service(ServiceId::Rshell),
            LibraryKind::Repository => Self::Service(ServiceId::Repository),
            _ => return None,
        })
    }
}
