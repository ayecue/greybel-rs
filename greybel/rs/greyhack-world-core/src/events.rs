//! Core events delivered to higher domains.

use greyhack_behaviour::LogAction;

use crate::ComputerId;

/// Owned login data published to packet-capture subscribers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginPacket {
    /// Computer the login comes from.
    pub source: ComputerId,
    /// Computer that was logged in to.
    pub destination: ComputerId,
    /// Address the login connected to.
    pub address: String,
    /// Port the login connected to.
    pub port: u16,
    /// User of the login.
    pub user: String,
    /// Password of the login.
    pub password: String,
}

impl greyhack_world_modules::Event for LoginPacket {}

/// A deletion that may start an administrator's active trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TraceAction {
    /// Computer whose file was deleted.
    pub remote: ComputerId,
    /// Number of bounces used by the deleting program.
    pub bounces: u32,
    /// Action written to the system log.
    pub action: LogAction,
    /// Terminal associated with the action.
    pub terminal: u64,
}

impl greyhack_world_modules::Event for TraceAction {}
