//! Conversions of smart appliance and traffic camera failures.

use super::{DeviceFailure, Failure, Outcome};

impl From<greyhack_world::ApplianceFailure> for DeviceFailure {
    fn from(failure: greyhack_world::ApplianceFailure) -> Self {
        use greyhack_world::ApplianceFailure as World;

        match failure {
            World::ComputerUnavailable => Self::ComputerUnavailable,
            World::NoInternet => Self::NoInternet,
            World::Malfunction => Self::Malfunction,
            World::SettingsLock => Self::Locked,
            World::InvalidSettings => Self::InvalidSettings,
            World::AlarmWarning => Self::AlarmWarning,
        }
    }
}

impl From<greyhack_world::TrafficFailure> for DeviceFailure {
    fn from(failure: greyhack_world::TrafficFailure) -> Self {
        use greyhack_world::TrafficFailure as World;

        match failure {
            World::ComputerUnavailable => Self::ComputerUnavailable,
            World::NoInternet => Self::NoInternet,
            World::MissingPoliceNetwork => Self::MissingPoliceNetwork,
            World::Cooldown => Self::Cooldown,
            World::CamerasOff => Self::CamerasDisabled,
            World::MissingWorkstation(name) => Self::MissingWorkstation(name),
            World::IncorrectPassword => Self::IncorrectPassword,
            World::InvalidPlate => Self::InvalidPlate,
            World::UnknownPlate => Self::UnknownPlate,
            World::OutOfSight => Self::NotLocated,
            World::CrownShield => Self::CrownShield,
        }
    }
}

impl From<Result<(), greyhack_world::ApplianceFailure>> for Outcome {
    fn from(result: Result<(), greyhack_world::ApplianceFailure>) -> Self {
        result.map_or_else(
            |failure| Self::Failure(Failure::Device(failure.into())),
            |()| Self::True,
        )
    }
}

impl From<Result<(), greyhack_world::TrafficFailure>> for Outcome {
    fn from(result: Result<(), greyhack_world::TrafficFailure>) -> Self {
        result.map_or_else(
            |failure| Self::Failure(Failure::Device(failure.into())),
            |()| Self::True,
        )
    }
}
