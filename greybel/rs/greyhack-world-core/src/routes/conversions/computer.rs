//! Conversions of account, process, network card, startup, and computer failures.

use greyhack_world_programs::protocol::{
    ComputerFailure, Failure, NetCardFailure, NetworkSwitch, Outcome, ProcessFailure,
    StartupFailure, UserFailure,
};

use super::IntoProtocol;

impl IntoProtocol for crate::UserFailure {
    type Output = UserFailure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        use crate::UserFailure as World;

        match failure {
            World::RootExists => UserFailure::RootExists,
            World::ReservedGuest => UserFailure::ReservedGuest,
            World::UserExists(user) => UserFailure::UserExists(user),
            World::TooManyUsers => UserFailure::TooManyUsers,
        }
    }
}

impl IntoProtocol for crate::ProcessFailure {
    type Output = ProcessFailure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        use crate::ProcessFailure as World;

        match failure {
            World::ComputerUnavailable => ProcessFailure::ComputerUnavailable,
            World::TooManyTerminals => ProcessFailure::TooManyTerminals,
            World::NoRam { remote } => ProcessFailure::NoRam { remote },
        }
    }
}

impl IntoProtocol for crate::NetCardFailure {
    type Output = NetCardFailure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        use crate::NetCardFailure as World;

        match failure {
            World::ComputerUnavailable => NetCardFailure::ComputerUnavailable,
            World::UnknownRouter => NetCardFailure::UnknownRouter,
            World::DisabledHomeNetwork => NetCardFailure::DisabledHomeNetwork,
            World::SelfGateway => NetCardFailure::SelfGateway,
            World::UnknownGateway(gateway) => NetCardFailure::UnknownGateway(gateway),
            World::AddressInUse(address) => NetCardFailure::AddressInUse(address),
            World::NoWifiCard => NetCardFailure::NoWifiCard,
            World::NoEthernetCard => NetCardFailure::NoEthernetCard,
        }
    }
}

impl IntoProtocol for crate::StartupFailure {
    type Output = StartupFailure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        use crate::StartupFailure as World;

        match failure {
            World::MissingKernelFile(path) => StartupFailure::MissingKernelFile(path.to_owned()),
            World::MissingGraphicsFile(path) => {
                StartupFailure::MissingGraphicsFile(path.to_owned())
            }
            World::MissingSystemFile(path) => StartupFailure::MissingSystemFile(path.to_owned()),
            World::InvalidFile(path) => StartupFailure::InvalidFile(path.to_owned()),
        }
    }
}

impl IntoProtocol for crate::ComputerFailure {
    type Output = Failure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        use crate::ComputerFailure as World;

        Failure::Computer(match failure {
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
            World::User(failure) => ComputerFailure::User(failure.into_protocol()),
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
            World::NetCard(failure) => ComputerFailure::NetCard(failure.into_protocol()),
            World::Refusal(refusal) => return Failure::Refusal(refusal.into_protocol()),
        })
    }
}

impl IntoProtocol for crate::NetworkSwitch {
    type Output = NetworkSwitch;

    fn into_protocol(self) -> Self::Output {
        let switch = self;

        use crate::SwitchResult as World;

        NetworkSwitch {
            outcome: match switch.result {
                World::Null => Outcome::Null,
                World::Connect => Outcome::True,
                World::Refusal(failure) => Outcome::Failure(failure.into_protocol()),
            },
            starts_wait: switch.starts_wait,
        }
    }
}
