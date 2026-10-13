use super::Kind;
use crate::protocol::ObjectId;
use crate::runtime::Handle;

/// Shell or computer payload (`HelperValComputer`) with its server object and FTP-access flag.
/// FTP access exposes fewer methods.
#[derive(Debug, Clone)]
pub(crate) struct Access {
    pub object: Handle,
    /// Server object for the shell's computer when distinct from `object`.
    /// A reverse shell has its own object (`Shell.reversePID`) but shares its computer object.
    pub computer: Option<Handle>,
    pub ftp: bool,
}

impl Access {
    /// Constructs a non-FTP payload for `object`.
    #[must_use]
    pub(crate) fn new(object: Handle) -> Self {
        Self {
            object,
            computer: None,
            ftp: false,
        }
    }

    /// Access level exposed by the shell's `host_computer`.
    #[must_use]
    pub(crate) fn host_computer(self) -> Self {
        Self {
            object: self.computer.unwrap_or(self.object),
            computer: None,
            ftp: self.ftp,
        }
    }

    /// Returns the server object's identity.
    pub(crate) fn reference(&self) -> ObjectId {
        self.object.reference()
    }

    /// Selects the restricted FTP map or unrestricted object map.
    pub(in crate::game) fn kind(&self, plain: Kind, ftp: Kind) -> Kind {
        if self.ftp { ftp } else { plain }
    }
}

/// Identifies an `adminComputer` by `lan_ip` behind `public_ip`.
#[derive(Debug, Clone)]
pub(crate) struct ComputerAddress {
    pub public_ip: String,
    pub lan_ip: String,
}

/// File payload (`HelperValFile`) with its server object and FTP-access flag.
/// The server object follows the file across moves.
#[derive(Debug, Clone)]
pub(crate) struct FileObject {
    pub object: Handle,
    pub ftp: bool,
}

impl FileObject {
    /// Returns the server file's identity.
    pub(crate) fn reference(&self) -> ObjectId {
        self.object.reference()
    }

    /// Constructs a payload for `object` with the same FTP access as this one.
    pub(crate) fn with_object(&self, object: Handle) -> Self {
        Self {
            object,
            ftp: self.ftp,
        }
    }
}

impl Handle {
    /// Returns the server object's identity.
    #[must_use]
    pub(crate) fn reference(&self) -> ObjectId {
        self.id()
    }
}
