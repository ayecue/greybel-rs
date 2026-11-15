//! Conversions of smart appliance and traffic camera failures.

pub(crate) use greyhack_world_core::routes::conversions::IntoProtocol;
pub(crate) use greyhack_world_programs::protocol::*;

impl IntoProtocol for crate::ApplianceFailure {
    type Output = DeviceFailure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        use crate::ApplianceFailure as World;

        match failure {
            World::ComputerUnavailable => DeviceFailure::ComputerUnavailable,
            World::NoInternet => DeviceFailure::NoInternet,
            World::Malfunction => DeviceFailure::Malfunction,
            World::SettingsLock => DeviceFailure::Locked,
            World::InvalidSettings => DeviceFailure::InvalidSettings,
            World::AlarmWarning => DeviceFailure::AlarmWarning,
        }
    }
}

impl IntoProtocol for crate::TrafficFailure {
    type Output = DeviceFailure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        use crate::TrafficFailure as World;

        match failure {
            World::ComputerUnavailable => DeviceFailure::ComputerUnavailable,
            World::NoInternet => DeviceFailure::NoInternet,
            World::MissingPoliceNetwork => DeviceFailure::MissingPoliceNetwork,
            World::Cooldown => DeviceFailure::Cooldown,
            World::CamerasOff => DeviceFailure::CamerasDisabled,
            World::MissingWorkstation(name) => DeviceFailure::MissingWorkstation(name),
            World::IncorrectPassword => DeviceFailure::IncorrectPassword,
            World::InvalidPlate => DeviceFailure::InvalidPlate,
            World::UnknownPlate => DeviceFailure::UnknownPlate,
            World::OutOfSight => DeviceFailure::NotLocated,
            World::CrownShield => DeviceFailure::CrownShield,
        }
    }
}
