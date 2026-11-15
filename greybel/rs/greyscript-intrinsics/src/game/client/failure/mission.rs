//! Grey Hack text for mission acceptance failures.

use greyhack_world_assets::GameTextTable;

use crate::protocol::MissionFailure;

/// Formats `failure` as the shop's window shows it (`MissionsHelperServer.AcceptMissionServerRpc`).
pub(super) fn mission_text(failure: MissionFailure) -> String {
    match failure {
        MissionFailure::NotAMissionShop | MissionFailure::NoMission => GameTextTable::english()
            .get("MISSION_ERROR")
            .unwrap_or("MISSION_ERROR")
            .to_owned(),
        MissionFailure::NoMailAccount => {
            "You must register an email account in order to accept a mission.".to_owned()
        }
    }
}
