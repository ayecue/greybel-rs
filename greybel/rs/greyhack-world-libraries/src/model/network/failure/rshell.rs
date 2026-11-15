use greyhack_world_core::ServiceFailure;

/// `metaxploit.rshell_client` connection errors (`MetaExploitIntrinsics.rshell_client`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RshellFailure {
    /// The address is not assigned to a network.
    UnknownAddress,
    /// The router does not forward the port to a computer.
    MissingServer,
    Service(ServiceFailure),
}
