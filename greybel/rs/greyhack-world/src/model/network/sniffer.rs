//! Active packet-capture state (`NetworkLan.SnifferStart` and `HelperSniffer`).

/// Captured network packet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnifferPacket {
    pub source: String,
    pub destination: String,
    pub protocol: String,
    /// User of the login the packet carries.
    pub user: String,
    /// Password of the login the packet carries.
    pub password: String,
    pub encrypted: bool,
    pub encoder: Option<String>,
}
