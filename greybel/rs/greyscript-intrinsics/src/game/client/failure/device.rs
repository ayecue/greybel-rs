//! Grey Hack failure text for smart appliances, police traffic networks, and clock phones.

use super::computer::COMPUTER_GONE;
use crate::protocol::{AlarmFailure, DeviceFailure};

/// Formats `SmartApplianceIntrinsics` and `TrafficNetIntrinsics` failures.
pub(super) fn device_text(intrinsic: &str, failure: &DeviceFailure) -> String {
    match failure {
        DeviceFailure::ComputerUnavailable => COMPUTER_GONE,
        DeviceFailure::NoInternet => "error: No internet access.",
        DeviceFailure::Malfunction => "error: Device hardware malfunction",
        DeviceFailure::Locked if intrinsic == "set_alarm" => {
            "set_alarm: denied: The alarm settings are locked for security reasons."
        }
        DeviceFailure::Locked => {
            "override_settings: denied: The settings are locked for security reasons."
        }
        DeviceFailure::InvalidSettings => {
            "override_settings: invalid power or temperature for this model. Check the device manual."
        }
        DeviceFailure::AlarmWarning => {
            "override_settings: warning: Unsafe settings applied. The safety alarm will shut down the appliance if \
             these values are reached."
        }
        DeviceFailure::MissingPoliceNetwork => {
            "error: This device is not registered on any police network"
        }
        DeviceFailure::Cooldown => {
            "Error: This user cannot access the global camera system for another 12 hours."
        }
        DeviceFailure::CamerasDisabled => {
            "Error: This network has disabled access to the traffic cameras."
        }
        DeviceFailure::MissingWorkstation(name) => {
            return format!(
                "error: Workstation not found. Unable to get the credentials info from {name}"
            );
        }
        DeviceFailure::IncorrectPassword => "error: Incorrect password. Access denied.",
        DeviceFailure::InvalidPlate => "error: Invalid license plate.",
        DeviceFailure::UnknownPlate => "locate_vehicle: License plate not found.",
        DeviceFailure::NotLocated => {
            "The vehicle could not be located in the camera system at this time."
        }
        DeviceFailure::CrownShield => {
            "The vehicle could not be located: CrownShield provides secure transport for this network administrator."
        }
    }
    .to_owned()
}

/// Formats `FileHelperServer.ModifyClockPhoneServerRpc` failures.
pub(super) fn alarm_text(failure: &AlarmFailure) -> String {
    match failure {
        AlarmFailure::IncorrectTime => {
            "Error: Incorrect time. A 24-hour format is expected.".to_owned()
        }
        AlarmFailure::IncompatibleDevice => {
            "Error: This program is not compatible with this device".to_owned()
        }
    }
}
