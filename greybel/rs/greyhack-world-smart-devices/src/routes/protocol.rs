//! Conversions of smart appliance and traffic camera failures.

pub(crate) use greyhack_world_core::routes::conversions::IntoProtocol;
pub(crate) use greyhack_world_programs::protocol::*;

impl IntoProtocol for crate::ApplianceFailure {
    type Output = DeviceFailure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        match failure {
            crate::ApplianceFailure::ComputerUnavailable => DeviceFailure::ComputerUnavailable,
            crate::ApplianceFailure::NoInternet => DeviceFailure::NoInternet,
            crate::ApplianceFailure::Malfunction => DeviceFailure::Malfunction,
            crate::ApplianceFailure::SettingsLock => DeviceFailure::Locked,
            crate::ApplianceFailure::InvalidSettings => DeviceFailure::InvalidSettings,
            crate::ApplianceFailure::AlarmWarning => DeviceFailure::AlarmWarning,
        }
    }
}

impl IntoProtocol for crate::TrafficFailure {
    type Output = DeviceFailure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        match failure {
            crate::TrafficFailure::ComputerUnavailable => DeviceFailure::ComputerUnavailable,
            crate::TrafficFailure::NoInternet => DeviceFailure::NoInternet,
            crate::TrafficFailure::MissingPoliceNetwork => DeviceFailure::MissingPoliceNetwork,
            crate::TrafficFailure::Cooldown => DeviceFailure::Cooldown,
            crate::TrafficFailure::CamerasOff => DeviceFailure::CamerasDisabled,
            crate::TrafficFailure::MissingWorkstation(name) => {
                DeviceFailure::MissingWorkstation(name)
            }
            crate::TrafficFailure::IncorrectPassword => DeviceFailure::IncorrectPassword,
            crate::TrafficFailure::InvalidPlate => DeviceFailure::InvalidPlate,
            crate::TrafficFailure::UnknownPlate => DeviceFailure::UnknownPlate,
            crate::TrafficFailure::OutOfSight => DeviceFailure::NotLocated,
            crate::TrafficFailure::CrownShield => DeviceFailure::CrownShield,
        }
    }
}
