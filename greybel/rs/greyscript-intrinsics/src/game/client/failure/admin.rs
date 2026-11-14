//! Grey Hack failure text for admin intrinsics.

use crate::protocol::AdminFailure;

/// Formats admin-intrinsic failures.
pub(super) fn admin_text(failure: &AdminFailure) -> String {
    match failure {
        AdminFailure::AddressTaken(address) => format!("Error: {address} is already in use"),
        AdminFailure::DomainTaken(domain) => format!("Error: {domain} is already in use"),
        AdminFailure::UnknownNetwork(address) => format!("Error: no network at {address}"),
        AdminFailure::LanAddressTaken => {
            "Error: that LAN address is taken or not in the network".to_owned()
        }
        AdminFailure::CorruptedRecords => "Can't apply changes. Corrupted database!".to_owned(),
        AdminFailure::MissingRecord => {
            "Can't apply changes. The entry no longer exists.".to_owned()
        }
        AdminFailure::WrongCameraPassword => {
            "Unable to access the camera: wrong password".to_owned()
        }
        AdminFailure::CamerasDisabled => "Temporarily disabled external camera access".to_owned(),
        AdminFailure::GuildNameTaken(name) => format!("Error: a guild is called {name} already"),
        AdminFailure::GuildAccountTaken(account) => {
            format!("Error: a guild holds shares under {account} already")
        }
        AdminFailure::UnknownGuild(name) => format!("Error: no guild is called {name}"),
        AdminFailure::InvalidPublicAddress => "Enter a valid public IP address.".to_owned(),
        AdminFailure::NoCrown => "Crown required".to_owned(),
        AdminFailure::CrownShieldActive => "Error: the guild has a CrownShield already".to_owned(),
        AdminFailure::InvalidCompany => "Error: invalid company".to_owned(),
        AdminFailure::UnknownCompany => "Error: company not found".to_owned(),
        AdminFailure::NotGuildCompany => {
            "Error: Admin Restore can only be configured for guild-owned companies.".to_owned()
        }
        AdminFailure::AdminRestoreLimit => "Error: This guild cannot disable Admin Restore for more companies at its current perk level.".to_owned(),
        AdminFailure::LibraryUpdateSkipped => {
            "Thanks for the warning, but I am not going to update them for now.".to_owned()
        }
        AdminFailure::UnknownLibrary(library) => format!("Error: Unknown lib {library}"),
        AdminFailure::UnreportableLibrary => "This library cannot be reported".to_owned(),
        AdminFailure::UnknownReportedZone => {
            "This memory zone does not exist in the current version of the library".to_owned()
        }
        AdminFailure::UnknownReportedValue { value, zone } => {
            format!("The unsecure value {value} does not exist in the memory zone {zone}")
        }
        AdminFailure::ReportPending => "Your previous report is still processing. Please wait a few minutes before sending a new report.".to_owned(),
    }
}
