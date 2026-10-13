//! Conversions of account, process, network card, startup, and computer failures.

use super::{
    ComputerFailure, Failure, NetCardFailure, NetworkSwitch, Outcome, ProcessFailure,
    StartupFailure, UserFailure,
};

impl From<greyhack_world::UserFailure> for UserFailure {
    fn from(failure: greyhack_world::UserFailure) -> Self {
        use greyhack_world::UserFailure as World;

        match failure {
            World::RootExists => Self::RootExists,
            World::ReservedGuest => Self::ReservedGuest,
            World::UserExists(user) => Self::UserExists(user),
            World::TooManyUsers => Self::TooManyUsers,
        }
    }
}

impl From<greyhack_world::ProcessFailure> for ProcessFailure {
    fn from(failure: greyhack_world::ProcessFailure) -> Self {
        use greyhack_world::ProcessFailure as World;

        match failure {
            World::ComputerUnavailable => Self::ComputerUnavailable,
            World::TooManyTerminals => Self::TooManyTerminals,
            World::NoRam { remote } => Self::NoRam { remote },
        }
    }
}

impl From<greyhack_world::NetCardFailure> for NetCardFailure {
    fn from(failure: greyhack_world::NetCardFailure) -> Self {
        use greyhack_world::NetCardFailure as World;

        match failure {
            World::ComputerUnavailable => Self::ComputerUnavailable,
            World::UnknownRouter => Self::UnknownRouter,
            World::DisabledHomeNetwork => Self::DisabledHomeNetwork,
            World::SelfGateway => Self::SelfGateway,
            World::UnknownGateway(gateway) => Self::UnknownGateway(gateway),
            World::AddressInUse(address) => Self::AddressInUse(address),
            World::NoWifiCard => Self::NoWifiCard,
            World::NoEthernetCard => Self::NoEthernetCard,
        }
    }
}

impl From<greyhack_world::StartupFailure> for StartupFailure {
    fn from(failure: greyhack_world::StartupFailure) -> Self {
        use greyhack_world::StartupFailure as World;

        match failure {
            World::MissingKernelFile(path) => Self::MissingKernelFile(path.to_owned()),
            World::MissingGraphicsFile(path) => Self::MissingGraphicsFile(path.to_owned()),
            World::MissingSystemFile(path) => Self::MissingSystemFile(path.to_owned()),
            World::InvalidFile(path) => Self::InvalidFile(path.to_owned()),
        }
    }
}

impl From<greyhack_world::ComputerFailure> for Failure {
    fn from(failure: greyhack_world::ComputerFailure) -> Self {
        use greyhack_world::ComputerFailure as World;

        Self::Computer(match failure {
            World::RootOnly => ComputerFailure::RootOnly,
            World::NoRebootPermission => ComputerFailure::NoRebootPermission,
            World::RemoteSafeMode => ComputerFailure::RemoteSafeMode,
            World::SystemProcess => ComputerFailure::SystemProcess,
            World::ScriptProcess => ComputerFailure::ScriptProcess,
            World::ForeignProcess { pid, user } => ComputerFailure::ForeignProcess { pid, user },
            World::UnknownUser(user) => ComputerFailure::UnknownUser(user),
            World::UnknownGroup { group, user } => ComputerFailure::UnknownGroup { group, user },
            World::RootDeletion => ComputerFailure::RootDeletion,
            World::UndeletableUser => ComputerFailure::UndeletableUser,
            World::User(failure) => ComputerFailure::User(failure.into()),
            World::GuestUser => ComputerFailure::GuestUser,
            World::MissingNetDevice => ComputerFailure::MissingNetDevice,
            World::WifiCardsOnly => ComputerFailure::WifiCardsOnly,
            World::EthernetCardsOnly => ComputerFailure::EthernetCardsOnly,
            World::ReconnectTooSoon => ComputerFailure::ReconnectTooSoon,
            World::RentedComputer => ComputerFailure::RentedComputer,
            World::DisabledNetwork => ComputerFailure::DisabledNetwork,
            World::UnknownWifi { bssid, essid } => ComputerFailure::UnknownWifi { bssid, essid },
            World::OutOfReach => ComputerFailure::OutOfReach,
            World::IncorrectWifiPassword => ComputerFailure::IncorrectWifiPassword,
            World::InvalidAddress => ComputerFailure::InvalidAddress,
            World::InvalidGateway => ComputerFailure::InvalidGateway,
            World::DifferentSubnets => ComputerFailure::DifferentSubnets,
            World::NetCard(failure) => ComputerFailure::NetCard(failure.into()),
            World::Refusal(refusal) => return Self::Refusal(refusal.into()),
        })
    }
}

impl From<Option<Result<(), greyhack_world::ComputerFailure>>> for Outcome {
    /// Converts a computer operation result: `None` once the computer is gone.
    fn from(result: Option<Result<(), greyhack_world::ComputerFailure>>) -> Self {
        match result {
            None => Self::Null,
            Some(Ok(())) => Self::True,
            Some(Err(failure)) => Self::Failure(failure.into()),
        }
    }
}

impl From<greyhack_world::NetworkSwitch> for NetworkSwitch {
    fn from(switch: greyhack_world::NetworkSwitch) -> Self {
        use greyhack_world::SwitchResult as World;

        Self {
            outcome: match switch.result {
                World::Null => Outcome::Null,
                World::Connect => Outcome::True,
                World::Refusal(failure) => Outcome::Failure(failure.into()),
            },
            starts_wait: switch.starts_wait,
        }
    }
}
