/// SMTP lookup errors from `PlayerUtils.GetSmtpServer`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmtpFailure {
    /// The address is not a valid IP address.
    InvalidAddress,
    /// The querying computer no longer exists.
    HostDown,
    /// No computer has the LAN address or occupies the forward destination.
    UnknownHost,
    /// The public address is not assigned to a network.
    UnknownAddress,
    /// The router does not have this external port.
    MissingPort(u16),
    /// The public port does not forward to the computer SMTP port.
    InvalidService,
    /// The computer does not run SMTP.
    MissingService,
}
