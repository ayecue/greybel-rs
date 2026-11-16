//! Computer file, user, process, and network errors. The runtime formats each error using the
//! intrinsic that detects it.

use greyhack_behaviour::AccountRefusal;

/// File or folder access, creation, and modification errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileFailure {
    /// The target computer no longer exists.
    ComputerUnavailable,
    /// No file exists at the path.
    MissingFile(String),
    /// No folder exists at the path.
    MissingFolder(String),
    /// The path identifies a file rather than a folder.
    NotAFolder(String),
    /// The path is invalid or identifies the root.
    InvalidPath,
    /// The user lacks required read or write permission.
    NoPermission,
    /// The file is protected by the system.
    SystemFile,
    /// The destination is protected; contains the source name.
    SystemFileTarget(String),
    /// The operation would replace a folder with a file or a file with a folder.
    KindMismatch,
    /// The disk lacks space for the copy.
    DiskFull,
    /// The folder has reached its entry limit.
    FolderFull,
    /// The file system has reached its node limit.
    TooManyFiles,
    /// The `chmod` value does not use Grey Hack's format.
    WrongFormat,
    /// A parent folder prevents traversal to the source.
    CannotAccess(String),
    /// The source or a skipped descendant cannot be read.
    CannotRead(String),
    /// The destination folder or replaced entry cannot be written.
    CannotWrite(String),
    /// The existing destination tree cannot be removed; contains the first denied path.
    CannotReplace(String),
    /// The copy would overwrite its own source.
    OverwriteSelf,
    /// The copy completed with skipped entries.
    PartialCopy,
    /// The deletion completed with skipped entries.
    PartialDelete,
    /// Deletion stopped before any change; contains the requested path.
    DeletePermission(String),
    /// Folder entries cannot be listed; contains the folder path.
    FolderReadPermission(String),
}

/// User creation errors (`ComputerIntrinsics.create_user`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserFailure {
    /// The username is reserved for root.
    RootExists,
    /// The username is reserved for guest.
    ReservedGuest,
    /// The username already exists.
    UserExists(String),
    /// The computer has reached its user limit.
    TooManyUsers,
}

impl From<AccountRefusal> for UserFailure {
    fn from(refusal: AccountRefusal) -> Self {
        match refusal {
            AccountRefusal::RootExists => Self::RootExists,
            AccountRefusal::ReservedGuest => Self::ReservedGuest,
            AccountRefusal::TooManyUsers => Self::TooManyUsers,
        }
    }
}

/// Process creation resource errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessFailure {
    /// The computer no longer exists.
    ComputerUnavailable,
    /// The computer has reached its terminal limit.
    TooManyTerminals,
    /// The computer lacks RAM for the process; `remote` identifies a computer other than the
    /// player's.
    NoRam { remote: bool },
}

/// Network connection errors (`NetworkLan.ConnectWifi`, `NetworkLan.ConnectEthernet`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetCardFailure {
    /// The computer no longer exists.
    ComputerUnavailable,
    /// The router or its network no longer exists.
    UnknownRouter,
    /// The computer network is disabled for nonpayment (`NetworkLan.ConnectEthernet`).
    DisabledHomeNetwork,
    /// The player has no rental home IP configured for Ethernet.
    NoRentalHome,
    /// The configured rental home IP has no ISP router.
    MissingIsp,
    /// The gateway address identifies the computer itself.
    SelfGateway,
    /// No network device has the gateway address.
    UnknownGateway(String),
    /// Another network device already uses the address.
    AddressInUse(String),
    /// No Wi-Fi card is installed.
    NoWifiCard,
    /// No Ethernet card is installed.
    NoEthernetCard,
}
