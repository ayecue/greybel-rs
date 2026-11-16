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

        match failure {
            crate::UserFailure::RootExists => UserFailure::RootExists,
            crate::UserFailure::ReservedGuest => UserFailure::ReservedGuest,
            crate::UserFailure::UserExists(user) => UserFailure::UserExists(user),
            crate::UserFailure::TooManyUsers => UserFailure::TooManyUsers,
        }
    }
}

impl IntoProtocol for crate::ProcessFailure {
    type Output = ProcessFailure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        match failure {
            crate::ProcessFailure::ComputerUnavailable => ProcessFailure::ComputerUnavailable,
            crate::ProcessFailure::TooManyTerminals => ProcessFailure::TooManyTerminals,
            crate::ProcessFailure::NoRam { remote } => ProcessFailure::NoRam { remote },
        }
    }
}

impl IntoProtocol for crate::NetCardFailure {
    type Output = NetCardFailure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        match failure {
            crate::NetCardFailure::ComputerUnavailable => NetCardFailure::ComputerUnavailable,
            crate::NetCardFailure::UnknownRouter => NetCardFailure::UnknownRouter,
            crate::NetCardFailure::DisabledHomeNetwork => NetCardFailure::DisabledHomeNetwork,
            crate::NetCardFailure::NoRentalHome => NetCardFailure::NoRentalHome,
            crate::NetCardFailure::MissingIsp => NetCardFailure::MissingIsp,
            crate::NetCardFailure::SelfGateway => NetCardFailure::SelfGateway,
            crate::NetCardFailure::UnknownGateway(gateway) => {
                NetCardFailure::UnknownGateway(gateway)
            }
            crate::NetCardFailure::AddressInUse(address) => NetCardFailure::AddressInUse(address),
            crate::NetCardFailure::NoWifiCard => NetCardFailure::NoWifiCard,
            crate::NetCardFailure::NoEthernetCard => NetCardFailure::NoEthernetCard,
        }
    }
}

impl IntoProtocol for crate::StartupFailure {
    type Output = StartupFailure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        match failure {
            crate::StartupFailure::MissingKernelFile(path) => {
                StartupFailure::MissingKernelFile(path.to_owned())
            }
            crate::StartupFailure::MissingGraphicsFile(path) => {
                StartupFailure::MissingGraphicsFile(path.to_owned())
            }
            crate::StartupFailure::MissingSystemFile(path) => {
                StartupFailure::MissingSystemFile(path.to_owned())
            }
            crate::StartupFailure::InvalidFile(path) => {
                StartupFailure::InvalidFile(path.to_owned())
            }
        }
    }
}

impl IntoProtocol for crate::ComputerFailure {
    type Output = Failure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        Failure::Computer(match failure {
            crate::ComputerFailure::RootOnly => ComputerFailure::RootOnly,
            crate::ComputerFailure::NoRebootPermission => ComputerFailure::NoRebootPermission,
            crate::ComputerFailure::RemoteSafeMode => ComputerFailure::RemoteSafeMode,
            crate::ComputerFailure::SystemProcess => ComputerFailure::SystemProcess,
            crate::ComputerFailure::ScriptProcess => ComputerFailure::ScriptProcess,
            crate::ComputerFailure::ForeignProcess { pid, user } => {
                ComputerFailure::ForeignProcess { pid, user }
            }
            crate::ComputerFailure::UnknownUser(user) => ComputerFailure::UnknownUser(user),
            crate::ComputerFailure::UnknownGroup { group, user } => {
                ComputerFailure::UnknownGroup { group, user }
            }
            crate::ComputerFailure::RootDeletion => ComputerFailure::RootDeletion,
            crate::ComputerFailure::UndeletableUser => ComputerFailure::UndeletableUser,
            crate::ComputerFailure::User(failure) => ComputerFailure::User(failure.into_protocol()),
            crate::ComputerFailure::GuestUser => ComputerFailure::GuestUser,
            crate::ComputerFailure::SmartphoneWifi => ComputerFailure::SmartphoneWifi,
            crate::ComputerFailure::MissingNetDevice => ComputerFailure::MissingNetDevice,
            crate::ComputerFailure::WifiCardsOnly => ComputerFailure::WifiCardsOnly,
            crate::ComputerFailure::EthernetCardsOnly => ComputerFailure::EthernetCardsOnly,
            crate::ComputerFailure::ReconnectTooSoon => ComputerFailure::ReconnectTooSoon,
            crate::ComputerFailure::RentedComputer => ComputerFailure::RentedComputer,
            crate::ComputerFailure::DisabledNetwork => ComputerFailure::DisabledNetwork,
            crate::ComputerFailure::UnknownWifi { bssid, essid } => {
                ComputerFailure::UnknownWifi { bssid, essid }
            }
            crate::ComputerFailure::OutOfReach => ComputerFailure::OutOfReach,
            crate::ComputerFailure::IncorrectWifiPassword => ComputerFailure::IncorrectWifiPassword,
            crate::ComputerFailure::InvalidAddress => ComputerFailure::InvalidAddress,
            crate::ComputerFailure::InvalidGateway => ComputerFailure::InvalidGateway,
            crate::ComputerFailure::DifferentSubnets => ComputerFailure::DifferentSubnets,
            crate::ComputerFailure::NetCard(failure) => {
                ComputerFailure::NetCard(failure.into_protocol())
            }
            crate::ComputerFailure::Refusal(refusal) => {
                return Failure::Refusal(refusal.into_protocol());
            }
        })
    }
}

impl IntoProtocol for crate::NetworkSwitch {
    type Output = NetworkSwitch;

    fn into_protocol(self) -> Self::Output {
        let switch = self;

        NetworkSwitch {
            outcome: match switch.result {
                crate::SwitchResult::Null => Outcome::Null,
                crate::SwitchResult::Connect => Outcome::True,
                crate::SwitchResult::Refusal(failure) => Outcome::Failure(failure.into_protocol()),
            },
            starts_wait: switch.starts_wait,
        }
    }
}
