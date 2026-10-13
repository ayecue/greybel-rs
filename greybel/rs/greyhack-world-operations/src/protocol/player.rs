//! Conversions of CTF, mission, and folder results.

use super::{CtfFailure, FolderChange, FolderLookup, MissionFailure};

impl From<greyhack_world::CtfFailure> for CtfFailure {
    fn from(failure: greyhack_world::CtfFailure) -> Self {
        use greyhack_world::CtfFailure as World;

        match failure {
            World::InvalidScore => Self::InvalidScore,
            World::UnknownEvent => Self::UnknownEvent,
            World::Incomplete => Self::Incomplete,
            World::RepeatScore => Self::RepeatScore,
        }
    }
}

impl From<greyhack_world::MissionFailure> for MissionFailure {
    fn from(failure: greyhack_world::MissionFailure) -> Self {
        use greyhack_world::MissionFailure as World;

        match failure {
            World::NotAMissionShop => Self::NotAMissionShop,
            World::NoMailAccount => Self::NoMailAccount,
            World::NoMission => Self::NoMission,
        }
    }
}

impl From<greyhack_world::FolderChange> for FolderChange {
    fn from(change: greyhack_world::FolderChange) -> Self {
        use greyhack_world::FolderLookup as World;

        Self {
            current_exists: change.current_exists,
            target: match change.target {
                World::Missing => FolderLookup::Missing,
                World::Denial => FolderLookup::Denial,
                World::Folder(path) => FolderLookup::Found(path),
            },
        }
    }
}
