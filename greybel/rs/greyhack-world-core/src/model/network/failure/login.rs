use super::{ConnectionFailure, ServiceFailure};

/// SSH and FTP login errors from `PlayerUtils.ConnectToService`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginFailure {
    /// The service is not SSH or FTP.
    InvalidService,
    Connection(ConnectionFailure),
    Service(ServiceFailure),
    /// The username or password is invalid.
    IncorrectPassword,
    /// The password is valid, but the account NPC is dead or fired (`Computer.ValidateUserPass`).
    AccountInactive,
}
