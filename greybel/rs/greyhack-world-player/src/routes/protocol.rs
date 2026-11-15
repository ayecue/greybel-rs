//! Conversions of CTF, mission, and folder results.

pub(crate) use greyhack_world_core::routes::conversions::IntoProtocol;
use greyhack_world_programs::protocol::{CtfFailure, FolderChange, FolderLookup, MissionFailure};

impl IntoProtocol for crate::CtfFailure {
    type Output = CtfFailure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        match failure {
            crate::CtfFailure::InvalidScore => CtfFailure::InvalidScore,
            crate::CtfFailure::UnknownEvent => CtfFailure::UnknownEvent,
            crate::CtfFailure::Incomplete => CtfFailure::Incomplete,
            crate::CtfFailure::RepeatScore => CtfFailure::RepeatScore,
            crate::CtfFailure::OwnEvent => CtfFailure::OwnEvent,
        }
    }
}

impl IntoProtocol for crate::MissionFailure {
    type Output = MissionFailure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        match failure {
            crate::MissionFailure::NotAMissionShop => MissionFailure::NotAMissionShop,
            crate::MissionFailure::NoMailAccount => MissionFailure::NoMailAccount,
            crate::MissionFailure::NoMission => MissionFailure::NoMission,
        }
    }
}

impl IntoProtocol for crate::FolderChange {
    type Output = FolderChange;

    fn into_protocol(self) -> Self::Output {
        let change = self;

        FolderChange {
            current_exists: change.current_exists,
            target: match change.target {
                crate::FolderLookup::Missing => FolderLookup::Missing,
                crate::FolderLookup::Denial => FolderLookup::Denial,
                crate::FolderLookup::Folder(path) => FolderLookup::Found(path),
            },
        }
    }
}
