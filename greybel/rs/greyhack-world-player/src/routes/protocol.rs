//! Conversions of CTF, mission, and folder results.

pub(crate) use greyhack_world_core::routes::conversions::IntoProtocol;
use greyhack_world_programs::protocol::{CtfFailure, FolderChange, FolderLookup, MissionFailure};

impl IntoProtocol for crate::CtfFailure {
    type Output = CtfFailure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        use crate::CtfFailure as World;

        match failure {
            World::InvalidScore => CtfFailure::InvalidScore,
            World::UnknownEvent => CtfFailure::UnknownEvent,
            World::Incomplete => CtfFailure::Incomplete,
            World::RepeatScore => CtfFailure::RepeatScore,
            World::OwnEvent => CtfFailure::OwnEvent,
        }
    }
}

impl IntoProtocol for crate::MissionFailure {
    type Output = MissionFailure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        use crate::MissionFailure as World;

        match failure {
            World::NotAMissionShop => MissionFailure::NotAMissionShop,
            World::NoMailAccount => MissionFailure::NoMailAccount,
            World::NoMission => MissionFailure::NoMission,
        }
    }
}

impl IntoProtocol for crate::FolderChange {
    type Output = FolderChange;

    fn into_protocol(self) -> Self::Output {
        let change = self;

        use crate::FolderLookup as World;

        FolderChange {
            current_exists: change.current_exists,
            target: match change.target {
                World::Missing => FolderLookup::Missing,
                World::Denial => FolderLookup::Denial,
                World::Folder(path) => FolderLookup::Found(path),
            },
        }
    }
}
